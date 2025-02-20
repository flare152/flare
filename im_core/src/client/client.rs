use crate::client::config::ClientConfig;
use crate::client::handlers::ClientMessageHandler;
use crate::client::message_handler::DefMessageHandler;
use crate::client::sys_handler::DefClientSystemHandler;
use crate::common::error::error::{FlareErr, Result};
use crate::connections::Connection;
use log::{error, warn};
use prost::Message as ProstMessage;
use protobuf_codegen::flare_gen::flare::net::LoginReq;
use protobuf_codegen::{Command, Message as ProtoMessage, Platform, Response};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, sleep, Instant};
use uuid;

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);
const RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
const MAX_RECONNECT_ATTEMPTS: u32 = 5;

#[derive(Clone, Debug)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Authenticated,
    Reconnecting { attempt: u32 },
}

impl fmt::Display for ClientState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Authenticating => write!(f, "Authenticating"),
            Self::Authenticated => write!(f, "Authenticated"),
            Self::Reconnecting { attempt } => write!(f, "Reconnecting (attempt {})", attempt),
        }
    }
}

pub struct Client<F>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<Box<dyn Connection>>> + Send + Sync>> + Send + Sync + 'static,
{
    config: ClientConfig,
    connector: Arc<F>,
    handler: Arc<ClientMessageHandler<DefClientSystemHandler, DefMessageHandler>>,
    state: Arc<Mutex<ClientState>>,
    conn: Arc<Mutex<Option<Box<dyn Connection>>>>,
    message_sender: mpsc::Sender<ProtoMessage>,
    last_pong: Arc<Mutex<Instant>>,
    is_running: Arc<Mutex<bool>>,
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<Response>>>>,
}

impl<F> Client<F>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<Box<dyn Connection>>> + Send + Sync>> + Send + Sync + 'static,
{
    pub fn new(
        connector: F,
        config: ClientConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let handler = Arc::new(ClientMessageHandler::default());
        let state = Arc::new(Mutex::new(ClientState::Disconnected));
        let conn = Arc::new(Mutex::new(None));
        let is_running = Arc::new(Mutex::new(true));
        let last_pong = Arc::new(Mutex::new(Instant::now()));
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));

        let client = Self {
            config,
            connector: Arc::new(connector),
            handler,
            state,
            conn,
            message_sender: tx,
            last_pong,
            is_running,
            pending_requests,
        };

        // 启动消息发送任务
        client.spawn_sender(rx);
        
        client
    }

    fn spawn_sender(&self, mut rx: mpsc::Receiver<ProtoMessage>) {
        let conn = self.conn.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if !*is_running.lock().await {
                    break;
                }
                
                if let Some(conn) = conn.lock().await.as_ref() {
                    match conn.send(msg).await {
                        Ok(_) => (),
                        Err(e) => {
                            error!("Failed to send message: {}", e);
                            continue;
                        }
                    }
                }
            }
        });
    }

    pub async fn connect(&self) -> Result<()> {
        self.set_state(ClientState::Connecting).await;
        
        let conn = (self.connector)().await?;
        *self.conn.lock().await = Some(conn);
        
        self.set_state(ClientState::Connected).await;
        
        // 发送认证消息
        self.authenticate().await?;
        
        // 启动接收任务
        self.spawn_receiver();
        
        // 启动保活任务
        self.spawn_keepalive();
        
        Ok(())
    }

    async fn authenticate(&self) -> Result<()> {
        self.set_state(ClientState::Authenticating).await;
        let req = LoginReq {
            user_id: self.config.user_id.clone(),
            platform: self.config.platform as i32,
            client_id: self.config.client_id.clone(),
            ..Default::default()
        };
        let auth_msg = ProtoMessage {
            command: Command::Login as i32,
            data: req.encode_to_vec(),
            ..Default::default()
        };
        
        self.send(auth_msg).await?;
        self.set_state(ClientState::Authenticated).await;
        Ok(())
    }

    fn spawn_receiver(&self) {
        let conn = self.conn.clone();
        let handler = self.handler.clone();
        let is_running = self.is_running.clone();
        let last_pong = self.last_pong.clone();
        let pending_requests = self.pending_requests.clone();
        let state = self.state.clone();
        
        tokio::spawn(async move {
            while let Some(conn) = conn.lock().await.as_ref() {
                if !*is_running.lock().await {
                    break;
                }

                match conn.receive().await {
                    Ok(msg) => {
                        if msg.command == Command::Pong as i32 {
                            *last_pong.lock().await = Instant::now();
                            continue;
                        }

                        let command = Command::try_from(msg.command).unwrap_or(Command::CmdUnknown);
                        if let Err(e) = handler.handle_command(command, msg.data).await {
                            error!("Error handling command: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        break;
                    }
                }
            }
        });
    }

    fn spawn_keepalive(&self) {
        let conn = self.conn.clone();
        let is_running = self.is_running.clone();
        let last_pong = self.last_pong.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(config.ping_interval);
            while *is_running.lock().await {
                interval.tick().await;
                
                if let Some(conn) = conn.lock().await.as_ref() {
                    // 检查上次 PONG 时间
                    let last = *last_pong.lock().await;
                    if last.elapsed() > config.pong_timeout {
                        warn!("No PONG received for {:?}, reconnecting", config.pong_timeout);
                        break;
                    }

                    // 发送 PING
                    let ping = ProtoMessage {
                        command: Command::Ping as i32,
                        ..Default::default()
                    };
                    
                    if let Err(e) = conn.send(ping).await {
                        error!("Failed to send PING: {}", e);
                        break;
                    }
                }
            }
        });
    }

    async fn set_state(&self, new_state: ClientState) {
        let mut state = self.state.lock().await;
        *state = new_state.clone();
        self.handler.handle_state_change(new_state).await;
    }

    /// 重连
    pub async fn reconnect(&self) -> Result<()> {
        let mut attempt = 0;
        while attempt < self.config.max_reconnect_attempts {
            self.set_state(ClientState::Reconnecting { attempt }).await;
            
            match self.connect().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    error!("Reconnection attempt {} failed: {}", attempt, e);
                    attempt += 1;
                    sleep(self.config.reconnect_interval).await;
                }
            }
        }
        
        Err(anyhow::anyhow!("Max reconnection attempts reached").into())
    }

    /// 发送消息
    pub async fn send(&self, msg: ProtoMessage) -> Result<()> {
        if !*self.is_running.lock().await {
            return Err(anyhow::anyhow!("Client is not running").into());
        }
        self.message_sender.send(msg).await
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }

    /// 发送消息并等待响应
    pub async fn send_wait(&self, mut msg: ProtoMessage) -> Result<Response> {
        // 生成唯一的请求ID
        msg.client_id = uuid::Uuid::new_v4().to_string();
        
        // 创建响应通道
        let (tx, rx) = oneshot::channel();
        
        // 保存请求
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(msg.client_id.clone(), tx);
        }
        
        // 发送消息
        self.send(msg).await?;
        
        // 等待响应
        match rx.await {
            Ok(response) => Ok(response),
            Err(_) => Err(anyhow::anyhow!("Response channel closed").into())
        }
    }

    /// 发送消息并等待响应，带超时
    pub async fn send_wait_timeout(&self, msg: ProtoMessage, timeout: Duration) -> Result<Response> {
        tokio::time::timeout(timeout, self.send_wait(msg))
            .await
            .map_err(|_| FlareErr::Timeout("Request timeout".to_string()))?
    }

    /// 关闭连接
    pub async fn close(&self) -> Result<()> {
        *self.is_running.lock().await = false;
        if let Some(conn) = self.conn.lock().await.as_ref() {
            conn.close().await?;
        }
        Ok(())
    }

    /// 获取当前状态
    pub async fn get_state(&self) -> ClientState {
        self.state.lock().await.clone()
    }

    /// 检查连接是否可用
    pub async fn is_connected(&self) -> bool {
        if !*self.is_running.lock().await {
            return false;
        }

        match self.get_state().await {
            ClientState::Connected | ClientState::Authenticated => {
                // 检查最后一次 PONG 时间
                let last_pong = *self.last_pong.lock().await;
                if last_pong.elapsed() > self.config.pong_timeout {
                    return false;
                }

                // 发送 PING 测试连接
                let ping = ProtoMessage {
                    command: Command::Ping as i32,
                    ..Default::default()
                };

                if let Some(conn) = self.conn.lock().await.as_ref() {
                    match conn.send(ping).await {
                        Ok(_) => true,
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// 获取连接状态详情
    pub async fn connection_status(&self) -> ConnectionStatus {
        ConnectionStatus {
            state: self.get_state().await,
            is_running: *self.is_running.lock().await,
            last_pong_elapsed: self.last_pong.lock().await.elapsed(),
            has_active_connection: self.conn.lock().await.is_some(),
        }
    }

    /// 等待连接就绪
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.is_connected().await {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        Err(anyhow::anyhow!("Connection timeout").into())
    }
}

/// 连接状态详情
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub state: ClientState,
    pub is_running: bool,
    pub last_pong_elapsed: Duration,
    pub has_active_connection: bool,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "State: {}, Running: {}, Last Pong: {:?} ago, Has Connection: {}",
            self.state,
            self.is_running,
            self.last_pong_elapsed,
            self.has_active_connection
        )
    }
}

impl<F> Drop for Client<F>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<Box<dyn Connection>>> + Send + Sync>> + Send + Sync + 'static,
{
    fn drop(&mut self) {
        futures::executor::block_on(async {
            *self.is_running.lock().await = false;
        });
    }
} 