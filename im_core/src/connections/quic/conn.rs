use crate::connections::connection::{Connection, ConnectionState};
use crate::common::error::error::{FlareErr, Result};
use log::debug;
use prost::Message as ProstMessage;
use protobuf_codegen::{Command, Message, Platform};
use quinn::{Connection as QuinnConnection, RecvStream, SendStream};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;
use std::pin::Pin;
use std::future::Future;
use async_trait::async_trait;

#[derive(Clone)]
pub struct QuicConnection {
    // 基础信息
    // 基础信息
    conn_id: String,
    protocol: String,
    remote_addr: String,
    // 连接状态
    state: Arc<Mutex<ConnectionState>>,
    // 最后活动时间
    last_active: Arc<Mutex<Instant>>,

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
        let (mut send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;

        // 等待服务器接受流
        send.write_all(b"hello").await
            .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;

        Ok(Self {
            conn_id: uuid::Uuid::new_v4().to_string(),
            protocol: "quic".to_string(),
            remote_addr,
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_active: Arc::new(Mutex::new(Instant::now())),
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
            protocol: "quic".to_string(),
            remote_addr,
            state: Arc::new(Mutex::new(ConnectionState::Connected)),
            last_active: Arc::new(Mutex::new(Instant::now())),
            conn: Arc::new(conn),
            send_stream: Arc::new(Mutex::new(send)),
            recv_stream: Arc::new(Mutex::new(recv)),
        })
    }

    async fn update_last_active(&self) {
        *self.last_active.lock().await = Instant::now();
    }
}

#[async_trait]
impl Connection for QuicConnection {
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
         // 检查 QUIC 连接状态
        return true
    }

    fn send(&self, msg: Message) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let send_stream = self.send_stream.clone();
        Box::pin(async move {
            debug!("Sending message: command={:?}, data_len={}", 
                Command::try_from(msg.command).unwrap_or(Command::CmdUnknown), 
                msg.data.len()
            );
            
            // 编码消息
            let mut data = Vec::new();
            msg.encode(&mut data)
                .map_err(|e| FlareErr::EncodeError(e))?;
            
            // 发送长度前缀
            let len = (data.len() as u32).to_be_bytes();
            send_stream.lock().await
                .write_all(&len)
                .await
                .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
            
            // 发送数据
            send_stream.lock().await
                .write_all(&data)
                .await
                .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
            
            self.update_last_active().await;
            Ok(())
        })
    }

    fn receive(&self) -> Pin<Box<dyn Future<Output = Result<Message>> + Send + '_>> {
        let recv_stream = self.recv_stream.clone();
        Box::pin(async move {
            // 读取长度前缀
            let mut len_bytes = [0u8; 4];
            recv_stream.lock().await
                .read_exact(&mut len_bytes)
                .await
                .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
            
            let len = u32::from_be_bytes(len_bytes) as usize;
            
            // 读取消息数据
            let mut data = vec![0u8; len];
            recv_stream.lock().await
                .read_exact(&mut data)
                .await
                .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
            
            // 解码消息
            let msg = Message::decode(&data[..])
                .map_err(|e| FlareErr::DecodeError(e))?;
            
            debug!("Received message: command={:?}, data_len={}", 
                Command::try_from(msg.command).unwrap_or(Command::CmdUnknown), 
                msg.data.len()
            );
            
            self.update_last_active().await;
            Ok(msg)
        })
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let state = self.state.clone();
        let send_stream = self.send_stream.clone();
        let conn = self.conn.clone();
        Box::pin(async move {
            *state.lock().await = ConnectionState::Disconnected;
            
            if let Ok(mut send) = send_stream.try_lock() {
                let _ = send.finish();
            }
            
            conn.close(0u32.into(), b"Normal closure");
            Ok(())
        })
    }

    fn clone_box(&self) -> Box<dyn Connection> {
        Box::new(self.clone())
    }
} 