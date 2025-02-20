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
        // 实现检查连接活跃状态的逻辑
        true
    }

    fn send(&self, msg: Message) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            // 实现发送逻辑
            Ok(())
        })
    }

    fn receive(&self) -> Pin<Box<dyn Future<Output = Result<Message>> + Send>> {
        Box::pin(async move {
            // 实现接收逻辑
            Ok(Message::default())
        })
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            // 实现关闭逻辑
            Ok(())
        })
    }

    fn clone_box(&self) -> Box<dyn Connection> {
        Box::new(self.clone())
    }
} 