use crate::connection::{Connection, ConnectionState};
use crate::error::{ConnectionError, Result};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use protobuf_codegen::{Command, Message, Platform};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_tungstenite::{tungstenite, WebSocketStream};
use log::{debug, warn};

#[derive(Clone)]
pub struct WsConnection<S> {
    // 基础信息
    conn_id: String,
    client_id: Arc<Mutex<String>>,
    user_id: Arc<Mutex<String>>,
    platform: Arc<Mutex<Platform>>,
    remote_addr: String,
    language: Arc<Mutex<Option<String>>>,

    // 连接状态
    state: Arc<Mutex<ConnectionState>>,
    last_active: Arc<Mutex<Instant>>,
    is_authenticated: Arc<Mutex<bool>>,

    // WebSocket 流
    writer: Arc<Mutex<SplitSink<WebSocketStream<S>, tungstenite::Message>>>,
    reader: Arc<Mutex<SplitStream<WebSocketStream<S>>>>,
}

impl<S> WsConnection<S>
where
    S: Send + Sync + Unpin + 'static,
    WebSocketStream<S>: Send + Sync + StreamExt<Item = std::result::Result<tungstenite::Message, tungstenite::Error>> + SinkExt<tungstenite::Message>,
{
    pub fn new(stream: WebSocketStream<S>, remote_addr: String) -> Self {
        let (writer, reader) = stream.split();
        Self {
            conn_id: uuid::Uuid::new_v4().to_string(),
            client_id: Arc::new(Mutex::new(String::new())),
            user_id: Arc::new(Mutex::new(String::new())),
            platform: Arc::new(Mutex::new(Platform::Unknown)),
            remote_addr,
            language: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_active: Arc::new(Mutex::new(Instant::now())),
            is_authenticated: Arc::new(Mutex::new(false)),
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
        }
    }

    async fn update_last_active(&self) {
        *self.last_active.lock().await = Instant::now();
    }
}

#[async_trait::async_trait]
impl<S> Connection for WsConnection<S>
where
    S: Send + Sync + Unpin + 'static,
    WebSocketStream<S>: Send + Sync + StreamExt<Item = std::result::Result<tungstenite::Message, tungstenite::Error>> + SinkExt<tungstenite::Message>,
{
    fn id(&self) -> &str {
        &self.conn_id
    }

    fn client_id(&self) -> &str {
        static EMPTY: &str = "";
        match self.client_id.try_lock() {
            Ok(guard) => {
                if guard.is_empty() {
                    EMPTY
                } else {
                    Box::leak(guard.clone().into_boxed_str())
                }
            }
            _ => EMPTY
        }
    }

    fn set_client_id(&self, client_id: String) {
        if let Ok(mut guard) = self.client_id.try_lock() {
            *guard = client_id;
        }
    }

    fn user_id(&self) -> &str {
        static EMPTY: &str = "";
        match self.user_id.try_lock() {
            Ok(guard) => {
                if guard.is_empty() {
                    EMPTY
                } else {
                    Box::leak(guard.clone().into_boxed_str())
                }
            }
            _ => EMPTY
        }
    }

    fn remote_addr(&self) -> &str {
        &self.remote_addr
    }

    fn platform(&self) -> Platform {
        self.platform.try_lock()
            .map(|guard| *guard)
            .unwrap_or(Platform::Unknown)
    }

    fn language(&self) -> Option<String> {
        self.language.try_lock()
            .map(|guard| guard.clone())
            .unwrap_or(None)
    }

    fn set_language(&self, language: String) {
        if let Ok(mut guard) = self.language.try_lock() {
            *guard = Some(language);
        }
    }

    fn set_user_info(&self, user_id: String, platform: Platform, language: String) {
        if let Ok(mut guard) = self.user_id.try_lock() {
            *guard = user_id;
        }
        if let Ok(mut guard) = self.platform.try_lock() {
            *guard = platform;
        }
        if let Ok(mut guard) = self.language.try_lock() {
            *guard = Some(language);
        }
    }

    fn state(&self) -> ConnectionState {
        self.state.try_lock()
            .map(|guard| *guard)
            .unwrap_or(ConnectionState::Error)
    }

    fn is_authenticated(&self) -> bool {
        self.is_authenticated.try_lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    fn set_authenticated(&self, value: bool) {
        if let Ok(mut guard) = self.is_authenticated.try_lock() {
            *guard = value;
        }
    }

    async fn send(&self, msg: Message) -> Result<()> {
        debug!("Sending message: command={:?}, data_len={}", 
            Command::try_from(msg.command).unwrap_or(Command::CmdUnknown), msg.data.len());
        
        let mut data = Vec::new();
        msg.encode(&mut data).map_err(ConnectionError::EncodeError)?;

        self.writer.lock().await
            .send(tungstenite::Message::Binary(data))
            .await
            .map_err(|_| ConnectionError::WebSocketError("Failed to send message".to_string()))?;

        self.update_last_active().await;
        Ok(())
    }

    async fn receive(&self) -> Result<Message> {
        if let Some(msg) = Pin::new(&mut *self.reader.lock().await).next().await {
            let msg = msg.map_err(ConnectionError::from)?;
            self.update_last_active().await;
            
            match msg {
                tungstenite::Message::Binary(data) => {
                    let msg = Message::decode(data.as_slice())
                        .map_err(ConnectionError::DecodeError)?;
                    debug!("Received message: command={:?}, data_len={}", 
                        Command::try_from(msg.command).unwrap_or(Command::CmdUnknown), msg.data.len());
                    Ok(msg)
                }
                tungstenite::Message::Ping(_) => {
                    debug!("Received ping message");
                    if let Err(_) = self.writer.lock().await
                        .send(tungstenite::Message::Pong(vec![]))
                        .await {
                        warn!("Failed to send pong");
                    }
                    Ok(Message::default())
                }
                tungstenite::Message::Pong(_) => {
                    debug!("Received pong message");
                    Ok(Message::default())
                }
                tungstenite::Message::Close(_) => {
                    debug!("Received close message");
                    Err(ConnectionError::ConnectionClosed)
                }
                _ => {
                    warn!("Received invalid message type");
                    Err(ConnectionError::InvalidMessageType)
                }
            }
        } else {
            warn!("Connection closed by peer");
            Err(ConnectionError::ConnectionClosed)
        }
    }

    async fn close(&self) -> Result<()> {
        *self.state.lock().await = ConnectionState::Disconnected;

        let close_frame = tungstenite::protocol::CloseFrame {
            code: tungstenite::protocol::frame::coding::CloseCode::Normal,
            reason: "Connection closed by client".into(),
        };

        if let Err(_) = self.writer.lock().await
            .send(tungstenite::Message::Close(Some(close_frame)))
            .await {
            debug!("Failed to send close frame");
        }

        Ok(())
    }
}