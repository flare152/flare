use crate::connection::{Connection, ConnectionState};
use crate::error::{ConnectionError, Result};
use log::{debug, info};
use prost::Message as ProstMessage;
use protobuf_codegen::{Command, Message, Platform};
use quinn::{Connection as QuinnConnection, RecvStream, SendStream};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct QuicConnection {
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

    // QUIC 连接
    conn: Arc<QuinnConnection>,
    send_stream: Arc<Mutex<SendStream>>,
    recv_stream: Arc<Mutex<RecvStream>>,
}

impl QuicConnection {
    pub async fn new(
        conn: QuinnConnection,
        remote_addr: String,
    ) -> Result<Self> {
        // 打开双向流
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;

        // 等待服务器接受流
        send.write_all(b"hello").await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;

        Ok(Self {
            conn_id: uuid::Uuid::new_v4().to_string(),
            client_id: Arc::new(Mutex::new(String::new())),
            user_id: Arc::new(Mutex::new(String::new())),
            platform: Arc::new(Mutex::new(Platform::Unknown)),
            remote_addr,
            language: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_active: Arc::new(Mutex::new(Instant::now())),
            is_authenticated: Arc::new(Mutex::new(false)),
            conn: Arc::new(conn),
            send_stream: Arc::new(Mutex::new(send)),
            recv_stream: Arc::new(Mutex::new(recv)),
        })
    }

    pub async fn with_streams(
        conn: QuinnConnection,
        send: SendStream,
        recv: RecvStream,
        remote_addr: String,
    ) -> Result<Self> {
        Ok(Self {
            conn_id: uuid::Uuid::new_v4().to_string(),
            client_id: Arc::new(Mutex::new(String::new())),
            user_id: Arc::new(Mutex::new(String::new())),
            platform: Arc::new(Mutex::new(Platform::Unknown)),
            remote_addr,
            language: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_active: Arc::new(Mutex::new(Instant::now())),
            is_authenticated: Arc::new(Mutex::new(false)),
            conn: Arc::new(conn),
            send_stream: Arc::new(Mutex::new(send)),
            recv_stream: Arc::new(Mutex::new(recv)),
        })
    }

    async fn update_last_active(&self) {
        *self.last_active.lock().await = Instant::now();
    }
}

#[async_trait::async_trait]
impl Connection for QuicConnection {
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
            _ => EMPTY,
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
            _ => EMPTY,
        }
    }

    fn remote_addr(&self) -> &str {
        &self.remote_addr
    }

    fn platform(&self) -> Platform {
        self.platform
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(Platform::Unknown)
    }

    fn language(&self) -> Option<String> {
        self.language.try_lock().map(|guard| guard.clone()).unwrap_or(None)
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
        self.state
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(ConnectionState::Error)
    }

    fn is_authenticated(&self) -> bool {
        self.is_authenticated
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    fn set_authenticated(&self, value: bool) {
        if let Ok(mut guard) = self.is_authenticated.try_lock() {
            *guard = value;
        }
    }

    async fn send(&self, msg: Message) -> Result<()> {
        debug!(
            "Sending message: command={:?}, data_len={}",
            Command::try_from(msg.command).unwrap_or(Command::CmdUnknown),
            msg.data.len()
        );

        let mut data = Vec::new();
        msg.encode(&mut data).map_err(ConnectionError::EncodeError)?;

        // 发送消息长度
        let len = data.len() as u32;
        let len_bytes = len.to_be_bytes();
        
        let mut stream = self.send_stream.lock().await;
        stream
            .write_all(&len_bytes)
            .await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;

        // 发送消息内容
        stream
            .write_all(&data)
            .await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;

        // 确保数据被发送
        stream.flush().await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;

        self.update_last_active().await;
        Ok(())
    }

    async fn receive(&self) -> Result<Message> {
        let mut stream = self.recv_stream.lock().await;

        // 读取消息长度
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        debug!("Receiving message with length: {}", len);

        // 读取消息内容
        let mut data = vec![0u8; len];
        stream
            .read_exact(&mut data)
            .await
            .map_err(|e| ConnectionError::WebSocketError(e.to_string()))?;

        let msg = Message::decode(data.as_slice()).map_err(ConnectionError::DecodeError)?;
        debug!(
            "Received message: command={:?}, data_len={}",
            Command::try_from(msg.command).unwrap_or(Command::CmdUnknown),
            msg.data.len()
        );

        self.update_last_active().await;
        Ok(msg)
    }

    async fn close(&self) -> Result<()> {
        *self.state.lock().await = ConnectionState::Disconnected;
        self.conn.close(0u32.into(), b"Normal closure");
        Ok(())
    }
} 