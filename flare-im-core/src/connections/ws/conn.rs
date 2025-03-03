use std::future::Future;
use crate::connections::connection::{Connection, ConnectionState};
use flare_core::error::{FlareErr, Result};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, warn};
use prost::Message as ProstMessage;
use flare_core::flare_net::net::{Command, Message, Platform};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio_tungstenite::{tungstenite, WebSocketStream};
use tokio_tungstenite::tungstenite::Bytes;

#[derive(Clone)]
pub struct WsConnection<S> {
    // 基础信息
    conn_id: String,
    protocol: String,
    remote_addr: String,
    // 连接状态
    state: Arc<Mutex<ConnectionState>>,
    // 最后活动时间
    last_active: Arc<Mutex<Instant>>,
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
            protocol: "websocket".to_string(),
            remote_addr,
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_active: Arc::new(Mutex::new(Instant::now())),
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
        }
    }

    /// 更新最后活动时间
    async fn update_last_active(&self) {
        *self.last_active.lock().await = Instant::now();
    }
}

#[async_trait]
impl<S> Connection for WsConnection<S>
where
    S: Send + Sync + Unpin + 'static,
    WebSocketStream<S>: Send + Sync + StreamExt<Item = std::result::Result<tungstenite::Message, tungstenite::Error>> + SinkExt<tungstenite::Message>,
{
    fn id(&self) -> &str {
        &self.conn_id
    }

    fn remote_addr(&self) -> &str {
        &self.remote_addr
    }

    fn platform(&self) -> Platform {
        Platform::Unknown
    }

    fn protocol(&self) -> &str {
        &self.protocol
    }

    async fn is_active(&self, timeout: Duration) -> bool {
        // 检查连接状态
        let state = *self.state.lock().await;
        if state != ConnectionState::Connected {
            return false;
        }

        // 检查最后活动时间
        let last_active = *self.last_active.lock().await;
        if last_active.elapsed() > timeout {
            return false;
        }

        // 尝试发送 ping 帧
        if let Err(_) = self.writer.lock().await
            .send(tungstenite::Message::Ping(Bytes::new()))
            .await {
            return false;
        }

        true
    }

    fn send(&self, msg: Message) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let writer = self.writer.clone();
        Box::pin(async move {
            // debug!("Sending message: command={:?}, data_len={}",
            //     Command::try_from(msg.command).unwrap_or(Command::CmdUnknown), msg.data.len());
            
            let mut data = Vec::new();
            msg.encode(&mut data).map_err(|e| FlareErr::ConnectionError(e.to_string()))?;

            writer.lock().await
                .send(tungstenite::Message::Binary(Bytes::from(data)))
                .await
                .map_err(|_| FlareErr::ConnectionError("Failed to send message".to_string()))?;

            self.update_last_active().await;
            Ok(())
        })
    }

    fn receive(&self) -> Pin<Box<dyn Future<Output = Result<Message>> + Send + '_>> {
        let reader = self.reader.clone();
        Box::pin(async move {
            if let Some(msg) = Pin::new(&mut *reader.lock().await).next().await {
                let msg = msg.map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
                self.update_last_active().await;
                
                match msg {
                    tungstenite::Message::Binary(data) => {
                        let msg = Message::decode(data)
                            .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
                        debug!("Received message: command={:?}, data_len={}", 
                            Command::try_from(msg.command).unwrap_or(Command::CmdUnknown), msg.data.len());
                        Ok(msg)
                    }
                    tungstenite::Message::Ping(_) => {
                        debug!("Received ping message");
                        if let Err(_) = self.writer.lock().await
                            .send(tungstenite::Message::Pong(Bytes::new()))
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
                        *self.state.lock().await = ConnectionState::Disconnected;
                        Err(FlareErr::ConnectionClosed)
                    }
                    _ => {
                        warn!("Received invalid message type");
                        *self.state.lock().await = ConnectionState::Disconnected;
                        Err(FlareErr::InvalidMessageType)
                    }
                }
            } else {
                warn!("Connection closed by peer");
                *self.state.lock().await = ConnectionState::Disconnected;
                Err(FlareErr::ConnectionClosed)
            }
        })
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
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
        })
    }

    fn clone_box(&self) -> Box<dyn Connection> {
        Box::new(WsConnection {
            conn_id: self.conn_id.clone(),
            protocol: self.protocol.clone(),
            remote_addr: self.remote_addr.clone(),
            state: self.state.clone(),
            last_active: self.last_active.clone(),
            writer: self.writer.clone(),
            reader: self.reader.clone(),
        })
    }
}