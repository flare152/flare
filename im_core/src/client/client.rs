use crate::client::config::ClientConfig;
use crate::client::handlers::ClientMessageHandler;
use crate::client::message_handler::DefMessageHandler;
use crate::client::sys_handler::DefClientSystemHandler;
use flare::error::FlareErr;
use flare::error::Result;
use crate::connections::Connection;
use log::{debug, error, warn};
use prost::Message as ProstMessage;
use protobuf_codegen::flare_gen::flare::net::LoginReq;
use protobuf_codegen::{Command, Message as ProtoMessage, Response};
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
    config: Arc<Mutex<ClientConfig>>,
    connector: Arc<Mutex<F>>,
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
            config: Arc::new(Mutex::new(config)),
            connector: Arc::new(Mutex::new(connector)),
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

    /// 连接到服务器
    pub async fn connect(&self) -> Result<()> {
        *self.state.lock().await = ClientState::Connecting;
        self.handler.handle_state_change(ClientState::Connecting).await;
        
        // 创建连接
        let connector = self.connector.lock().await;
        let new_conn = (connector)().await?;
        *self.conn.lock().await = Some(new_conn);

        // 启动消息接收循环
        self.spawn_receiver();
        
        // 进行认证
        self.authenticate().await?;

        // 认证成功后启动心跳检测
        self.spawn_keepalive();
        
        // 更新状态
        self.set_state(ClientState::Connected).await;
        Ok(())
    }

    /// 重连
    pub async fn reconnect(&self) -> Result<()> {
        let mut attempt = 0;
        while attempt < self.config.lock().await.max_reconnect_attempts {
            self.set_state(ClientState::Reconnecting { attempt }).await;
            
            match self.connect().await {
                Ok(()) => {
                    // 重连成功后重新进行认证
                    match self.authenticate().await {
                        Ok(()) => {
                            // 认证成功后重新启动心跳检测
                            self.spawn_keepalive();
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Authentication failed after reconnection: {}", e);
                            attempt += 1;
                        }
                    }
                }
                Err(e) => {
                    error!("Reconnection attempt {} failed: {}", attempt, e);
                    attempt += 1;
                }
            }
            sleep(self.config.lock().await.reconnect_interval).await;
        }
        
        self.set_state(ClientState::Disconnected).await;
        Err(anyhow::anyhow!("Max reconnection attempts reached").into())
    }

    /// 关闭连接
    pub async fn close(&self) -> Result<()> {
        *self.is_running.lock().await = false;
        if let Some(conn) = self.conn.lock().await.as_ref() {
            conn.close().await?;
        }
        Ok(())
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

    /// 获取当前连接
    pub async fn get_connection(&self) -> Result<Box<dyn Connection>> {
        if let Some(conn) = &*self.conn.lock().await {
            Ok(conn.clone_box())
        } else {
            Err(FlareErr::ConnectionNotFound)
        }
    }

    /// 更新连接
    pub async fn update_connection(&self, connection: Box<dyn Connection>, new_config: ClientConfig) -> Result<()> {
        // 更新连接
        let mut conn = self.conn.lock().await;
        *conn = Some(connection);
        drop(conn);

        // 更新配置
        let mut config = self.config.lock().await;
        *config = new_config;
        drop(config);

        // 设置状态为连接中
        self.set_state(ClientState::Connecting).await;

        // 启动消息接收循环
        self.spawn_receiver();

        // 启动心跳检测
        self.spawn_keepalive();

        // 进行认证
        self.authenticate().await?;

        // 设置状态为已连接
        self.set_state(ClientState::Connected).await;

        // 等待连接就绪
        self.wait_ready(Duration::from_secs(5)).await?;

        Ok(())
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
    pub async fn send_wait(&self, msg: ProtoMessage) -> Result<Response> {
        // 创建一个新的可变消息
        let mut new_msg = msg;
        new_msg.client_id = uuid::Uuid::new_v4().to_string();

        // 创建响应通道
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(new_msg.client_id.clone(), tx);
        }

        // 发送消息
        self.send(new_msg).await?;
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
            .map_err(|_| FlareErr::ConnectionError("Request timeout".to_string()))?
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
                let last_pong = *self.last_pong.lock().await;
                if last_pong.elapsed() > self.config.lock().await.pong_timeout {
                    return false;
                }

                let ping = ProtoMessage {
                    command: Command::Ping as i32,
                    ..Default::default()
                };

                let conn_ref = Self::get_connection_ref(&self.conn).await;
                if let Some(conn_ref) = conn_ref {
                    match conn_ref.send(ping).await {
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

    // 认证相关
    async fn authenticate(&self) -> Result<()> {
        self.set_state(ClientState::Authenticating).await;
        let conf = self.config.lock().await;
        let req = LoginReq {
            user_id: conf.user_id.clone(),
            platform: conf.platform as i32,
            client_id: conf.client_id.clone(),
            token: conf.auth_token.clone(),
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

    // 状态管理
    async fn set_state(&self, new_state: ClientState) {
        let mut state = self.state.lock().await;
        *state = new_state.clone();
        self.handler.handle_state_change(new_state).await;
    }

    // 连接管理
    async fn get_connection_ref(conn: &Arc<Mutex<Option<Box<dyn Connection>>>>) -> Option<Arc<Box<dyn Connection>>> {
        conn.lock().await.as_ref().map(|c| Arc::new(c.clone_box()))
    }

    // 后台任务
    fn spawn_sender(&self, mut rx: mpsc::Receiver<ProtoMessage>) {
        let conn = self.conn.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            let mut conn_ref = None;
            let mut msg_buffer = Vec::with_capacity(32); // 消息缓冲区
            let mut flush_timer = tokio::time::interval(Duration::from_millis(10)); // 定时刷新
            
            loop {
                tokio::select! {
                    // 接收消息
                    msg = rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if !*is_running.lock().await {
                                    warn!("Client is not running, skipping message send.");
                                    break;
                                }
                                msg_buffer.push(msg);
                                
                                // 缓冲区满时立即发送
                                if msg_buffer.len() >= 32 {
                                    if let Err(e) = Self::flush_messages(&mut conn_ref, &conn, &mut msg_buffer).await {
                                        error!("Failed to flush messages: {}", e);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    
                    // 定时刷新缓冲区
                    _ = flush_timer.tick() => {
                        if !msg_buffer.is_empty() {
                            if let Err(e) = Self::flush_messages(&mut conn_ref, &conn, &mut msg_buffer).await {
                                error!("Failed to flush messages: {}", e);
                            }
                        }
                    }
                }
            }
            
            // 退出前确保发送所有消息
            if !msg_buffer.is_empty() {
                if let Err(e) = Self::flush_messages(&mut conn_ref, &conn, &mut msg_buffer).await {
                    error!("Failed to flush remaining messages: {}", e);
                }
            }
        });
    }

    fn spawn_receiver(&self) {
        let conn = self.conn.clone();
        let handler = self.handler.clone();
        let is_running = self.is_running.clone();
        let last_pong = self.last_pong.clone();
        let pending_requests = self.pending_requests.clone();

        tokio::spawn(async move {
            while *is_running.lock().await {
                if let Some(conn_ref) = Self::get_connection_ref(&conn).await {
                    match conn_ref.receive().await {
                        Ok(msg) => {
                            // 处理 PONG 消息
                            if msg.command == Command::Pong as i32 {
                                *last_pong.lock().await = Instant::now();
                                continue;
                            }
                            // 处理服务端Ping
                            if msg.command == Command::Ping as i32 {
                                if let Err(e) = conn_ref.send(ProtoMessage {
                                    command: Command::Pong as i32,
                                    ..Default::default()
                                }).await {
                                    error!("Failed to send Pong message: {}", e);
                                }
                                continue;
                            }
                            // 处理响应消息
                            if msg.command == Command::ServerResponse as i32 {
                                if let Ok(response) = Response::decode(&msg.data[..]) {
                                    // 检查是否有待处理的请求
                                    let mut pending = pending_requests.lock().await;
                                    if let Some(tx) = pending.remove(&msg.client_id) {
                                        let _ = tx.send(response.clone());
                                    }
                                    // 通知消息处理器
                                    handler.on_response(&response).await;
                                }
                                continue;
                            }

                            // 处理其他消息
                            if let Ok(command) = Command::try_from(msg.command) {
                                if let Err(e) = handler.handle_command(command, msg.data).await {
                                    error!("Failed to handle command: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to receive message: {}", e);
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }

    fn spawn_keepalive(&self) {
        let conn = self.conn.clone();
        let is_running = self.is_running.clone();
        let state = self.state.clone();
        let handler = self.handler.clone();
        let config = Arc::clone(&self.config);
        
        tokio::spawn(async move {
            let config = config.lock().await;
            let mut interval = interval(config.ping_interval);
            let reconnect_interval = config.reconnect_interval;
            drop(config);

            while *is_running.lock().await {
                interval.tick().await;

                // 检查连接状态
                if !matches!(*state.lock().await, ClientState::Connected | ClientState::Authenticated) {
                    // 如果连接断开，等待一段时间后尝试重连
                    sleep(reconnect_interval).await;
                    handler.handle_state_change(ClientState::Reconnecting { attempt: 0 }).await;
                    continue;
                }
                
                // 发送心跳包
                if let Some(conn_ref) = Self::get_connection_ref(&conn).await {
                    let ping_msg = ProtoMessage {
                        command: Command::Ping as i32,
                        ..Default::default()
                    };
                    
                    if let Err(e) = conn_ref.send(ping_msg).await {
                        error!("Failed to send ping: {}", e);
                        *state.lock().await = ClientState::Disconnected;
                        handler.handle_state_change(ClientState::Disconnected).await;
                    }
                }
            }
        });
    }

    // 消息处理
    async fn flush_messages(
        conn_ref: &mut Option<Arc<Box<dyn Connection>>>,
        conn: &Arc<Mutex<Option<Box<dyn Connection>>>>,
        msg_buffer: &mut Vec<ProtoMessage>
    ) -> Result<()> {
        // 确保连接可用
        if conn_ref.is_none() {
            *conn_ref = Self::get_connection_ref(conn).await;
        }

        if let Some(ref conn) = conn_ref {
            let mut failed = false;
            
            // 批量发送所有消息
            for msg in msg_buffer.drain(..) {
                debug!("Sending message: {:?}", msg);
                if let Err(e) = conn.send(msg).await {
                    error!("Failed to send message: {}", e);
                    failed = true;
                    break;
                }
            }

            if failed {
                *conn_ref = None; // 发送失败时清除连接引用
                msg_buffer.clear(); // 清空缓冲区
            }
        }
        
        Ok(())
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