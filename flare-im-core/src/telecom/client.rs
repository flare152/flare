use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use quinn::Endpoint;
use crate::client::client::{Client, ClientState};
use crate::client::config::ClientConfig;
use crate::client::handlers::ClientMessageHandler;
use crate::client::sys_handler::ClientSystemHandler;
use crate::client::message_handler::MessageHandler;
use flare_core::error::{Result, FlareErr};
use crate::connections::{Connection, WsConnection, QuicConnection};
use log::{info, debug, error};
use std::net::SocketAddr;
use std::pin::Pin;
use flare_core::flare_net::net::{Message, Platform, Response};
use crate::connections::quic_conf::create_client_config;
use std::time::Instant;

pub enum Protocol {
    Auto,
    WebSocket,
    Quic,
}
/// 连接信息结构体
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub remote_addr: String,
    pub platform: Platform,
    pub protocol: String,
    pub is_active: bool,
    pub state: ClientState,
}


pub struct FlareClient<S, M> 
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    client: Arc<Client<Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<Box<dyn Connection>>> + Send + Sync>> + Send + Sync>>>,
    ws_url: String,
    quic_addr: String,
    quic_server_name: String,
    quic_cert_path: String,
    quic_is_test: bool,
    config: ClientConfig,
    handler: Arc<ClientMessageHandler<S, M>>,
    state: Arc<Mutex<ClientState>>,
    reconnect_attempts: Arc<Mutex<u32>>,
    is_reconnecting: Arc<Mutex<bool>>,
    protocol: Protocol,
}

impl<S, M> FlareClient<S, M>
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    pub fn new(
        ws_url: String,
        quic_addr: String,
        quic_server_name: String,
        quic_cert_path: String,
        quic_is_test: bool,
        config: ClientConfig,
        handler: ClientMessageHandler<S, M>,
        protocol: Protocol,
    ) -> Self {
        Self {
            client: Arc::new(Client::new(
                Box::new(|| Box::pin(async { 
                    Err(FlareErr::ConnectionError("No connection established".to_string())) 
                })),
                config.clone(),
            )),
            ws_url,
            quic_addr,
            quic_server_name,
            quic_cert_path,
            quic_is_test,
            config,
            handler: Arc::new(handler),
            state: Arc::new(Mutex::new(ClientState::Disconnected)),
            reconnect_attempts: Arc::new(Mutex::new(0)),
            is_reconnecting: Arc::new(Mutex::new(false)),
            protocol,
        }
    }

    pub fn builder() -> FlareClientBuilder<S, M> {
        FlareClientBuilder::new()
    }

    /// 连接服务器
    pub async fn connect(&mut self) -> Result<()> {
        self.set_state(ClientState::Connecting).await;

        match self.try_connect().await {
            Ok(()) => {
                self.set_state(ClientState::Connected).await;
                // 启动连接监控
                self.spawn_connection_monitor();
                Ok(())
            }
            Err(e) => {
                self.set_state(ClientState::Disconnected).await;
                Err(e)
            }
        }
    }

    /// 重连方法
    pub async fn reconnect(&mut self) -> Result<()> {
        if *self.is_reconnecting.lock().await {
            return Ok(());
        }
        
        *self.is_reconnecting.lock().await = true;
        
        while {
            let attempts = *self.reconnect_attempts.lock().await;
            attempts < self.config.max_reconnect_attempts
        } {
            let current_attempt = {
                let mut attempts = self.reconnect_attempts.lock().await;
                *attempts += 1;
                *attempts
            };
            
            self.set_state(ClientState::Reconnecting { attempt: current_attempt }).await;
            
            match self.try_connect().await {
                Ok(()) => {
                    *self.reconnect_attempts.lock().await = 0;
                    *self.is_reconnecting.lock().await = false;
                    self.set_state(ClientState::Connected).await;
                    return Ok(());
                }
                Err(e) => {
                    error!("Reconnection attempt {} failed: {}", current_attempt, e);
                    tokio::time::sleep(self.config.reconnect_interval).await;
                }
            }
        }

        *self.is_reconnecting.lock().await = false;
        self.set_state(ClientState::Disconnected).await;
        Err(FlareErr::ConnectionError("Max reconnection attempts reached".to_string()))
    }

    /// 关闭连接
    pub async fn close(&self) -> Result<()> {
        self.client.close().await
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        matches!(*self.state.lock().await, ClientState::Connected | ClientState::Authenticated)
    }

    /// 等待连接就绪，带超时
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            match *self.state.lock().await {
                ClientState::Connected | ClientState::Authenticated => {
                    // 检查连接是否真正可用
                    if self.is_connection_active(Duration::from_secs(1)).await {
                        return Ok(());
                    }
                }
                ClientState::Disconnected => {
                    return Err(FlareErr::ConnectionError("Connection disconnected".to_string()));
                }
                _ => {}
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(FlareErr::ConnectionError("Connection timeout".to_string()))
    }

    /// 检查连接是否活跃
    pub async fn is_connection_active(&self, timeout: Duration) -> bool {
        if let Ok(conn) = self.client.get_connection().await {
            conn.is_active(timeout).await
        } else {
            false
        }
    }

    /// 发送消息
    pub async fn send(&self, msg: Message) -> Result<()> {
        self.client.send(msg).await
    }
    /// 发送消息并等待响应
    pub async fn send_wait(&self, msg: Message) -> Result<Response> {
        self.client.send_wait(msg).await
    }
    /// 发送消息并等待响应，带超时
    pub async fn send_wait_timeout(&self, msg: Message, timeout: Duration) -> Result<Response> {
        self.client.send_wait_timeout(msg, timeout).await
    }

    /// 获取当前状态
    pub async fn get_state(&self) -> ClientState {
        self.state.lock().await.clone()
    }

    /// 获取连接ID
    pub async fn connection_id(&self) -> Option<String> {
        if let Ok(conn) = self.client.get_connection().await {
            Some(conn.id().to_string())
        } else {
            None
        }
    }

    /// 获取远程地址
    pub async fn remote_addr(&self) -> Option<String> {
        if let Ok(conn) = self.client.get_connection().await {
            Some(conn.remote_addr().to_string())
        } else {
            None
        }
    }

    /// 获取平台信息
    pub async fn platform(&self) -> Option<Platform> {
        if let Ok(conn) = self.client.get_connection().await {
            Some(conn.platform())
        } else {
            None
        }
    }

    /// 获取协议名称
    pub async fn protocol_name(&self) -> Option<String> {
        if let Ok(conn) = self.client.get_connection().await {
            Some(conn.protocol().to_string())
        } else {
            None
        }
    }

    /// 获取连接详细信息
    pub async fn connection_info(&self) -> Option<ConnectionInfo> {
        if let Ok(conn) = self.client.get_connection().await {
            Some(ConnectionInfo {
                id: conn.id().to_string(),
                remote_addr: conn.remote_addr().to_string(),
                platform: conn.platform(),
                protocol: conn.protocol().to_string(),
                is_active: conn.is_active(Duration::from_secs(30)).await,
                state: self.get_state().await,
            })
        } else {
            None
        }
    }

    // 连接 QUIC
    async fn connect_quic(&self) -> Result<Box<dyn Connection>> {
        let addr = self.quic_addr.parse::<SocketAddr>()
            .map_err(|e| FlareErr::ConnectionError(format!("Invalid QUIC address: {}", e)))?;

        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| FlareErr::ConnectionError(format!("Failed to create QUIC endpoint: {}", e)))?;
        let client_config = create_client_config(self.quic_cert_path.as_str(),self.quic_is_test)
            .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
        endpoint.set_default_client_config(client_config);

        let connecting = endpoint.connect(addr, &self.quic_server_name)
            .map_err(|e| FlareErr::ConnectionError(format!("Failed to connect QUIC: {}", e)))?;

        let connection = connecting.await
            .map_err(|e| FlareErr::ConnectionError(format!("QUIC connection failed: {}", e)))?;

        let quic_conn = QuicConnection::connect(connection, addr.to_string()).await?;
        Ok(Box::new(quic_conn))
    }

    // 连接 WebSocket
    async fn connect_websocket(&self) -> Result<Box<dyn Connection>> {
        let url = url::Url::parse(&self.ws_url)
            .map_err(|e| FlareErr::ConnectionError(format!("Invalid WebSocket URL: {}", e)))?;

        // 将 url::Url 转换为字符串
        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| FlareErr::ConnectionError(format!("WebSocket connection failed: {}", e)))?;

        Ok(Box::new(WsConnection::new(ws_stream, "websocket".to_string())))
    }

    // 实际的连接尝试逻辑
    async fn try_connect(&mut self) -> Result<()> {
        match self.protocol {
            Protocol::Auto => {
                self.try_connect_auto().await
            }
            Protocol::WebSocket => {
                info!("Using WebSocket protocol");
                match self.connect_websocket().await {
                    Ok(conn) => {
                        self.update_client_connector(conn).await
                    }
                    Err(e) => {
                        error!("WebSocket connection failed: {}", e);
                        Err(e)
                    }
                }
            }
            Protocol::Quic => {
                info!("Using QUIC protocol");
                match self.connect_quic().await {
                    Ok(conn) => {
                        self.update_client_connector(conn).await
                    }
                    Err(e) => {
                        error!("QUIC connection failed: {}", e);
                        Err(e)
                    }
                }
            }
        }
    }

    // 竞速连接逻辑
    async fn try_connect_auto(&mut self) -> Result<()> {
        let quic_future = self.connect_quic();
        let ws_future = self.connect_websocket();
        let client = Arc::clone(&self.client);
        let config = self.config.clone();

        let update_connector = move |conn| {
            let client = Arc::clone(&client);
            let config = config.clone();
            async move {
                client.update_connection(conn, config).await
            }
        };

        let mut quic_task = Box::pin(quic_future);
        let mut ws_task = Box::pin(ws_future);

        tokio::select! {
            quic_result = &mut quic_task => {
                match quic_result {
                    Ok(conn) => {
                        info!("QUIC connection established");
                        update_connector(conn).await?;
                    }
                    Err(e) => {
                        debug!("QUIC connection failed: {}, falling back to WebSocket", e);
                        if let Ok(conn) = ws_task.await {
                            info!("WebSocket connection established");
                            update_connector(conn).await?;
                        } else {
                            return Err(FlareErr::ConnectionError("All connection attempts failed".to_string()));
                        }
                    }
                }
            }
            ws_result = &mut ws_task => {
                // 即使 WebSocket 先连接成功，也要等待一段时间看 QUIC 是否能连接
                tokio::select! {
                    quic_result = tokio::time::timeout(Duration::from_secs(1), &mut quic_task) => {
                        match quic_result {
                            Ok(Ok(conn)) => {
                                info!("QUIC connection established (preferred)");
                                update_connector(conn).await?;
                            }
                            _ => {
                                // QUIC 超时或失败，使用 WebSocket
                                if let Ok(conn) = ws_result {
                                    info!("WebSocket connection established (fallback)");
                                    update_connector(conn).await?;
                                } else {
                                    return Err(FlareErr::ConnectionError("All connection attempts failed".to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // 更新客户端连接器
    async fn update_client_connector(&mut self, connection: Box<dyn Connection>) -> Result<()> {
        let conn = connection.clone_box();
        self.client = Arc::new(Client::new(
            Box::new(move || {
                let conn = conn.clone_box();
                Box::pin(async move { Ok(conn) })
            }),
            self.config.clone(),
        ));

        // 连接并等待认证
        self.client.connect().await?;
        self.client.wait_ready(Duration::from_secs(5)).await?;

        Ok(())
    }

    // 状态变更方法
    async fn set_state(&self, new_state: ClientState) {
        let mut state = self.state.lock().await;
        *state = new_state.clone();
        self.handler.handle_state_change(new_state).await;
    }

    // 启动连接监控
    fn spawn_connection_monitor(&self) {
        let client = self.client.clone();
        let state = self.state.clone();
        let handler = self.handler.clone();
        let config = self.config.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let is_reconnecting = self.is_reconnecting.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // 检查连接状态
                if let Ok(current_state) = state.try_lock() {
                    match *current_state {
                        ClientState::Connected | ClientState::Authenticated => {
                            // 检查连接是否存活
                            if !client.is_connected().await {
                                drop(current_state); // 释放锁
                                
                                // 尝试重连
                                if !*is_reconnecting.lock().await {
                                    let mut attempts = reconnect_attempts.lock().await;
                                    if *attempts < config.max_reconnect_attempts {
                                        *attempts += 1;
                                        drop(attempts);
                                        
                                        // 通知状态变化
                                        handler.handle_state_change(ClientState::Reconnecting { 
                                            attempt: *reconnect_attempts.lock().await 
                                        }).await;
                                        
                                        // 执行重连
                                        *is_reconnecting.lock().await = true;
                                        if let Err(e) = client.reconnect().await {
                                            error!("Reconnection failed: {}", e);
                                            handler.handle_state_change(ClientState::Disconnected).await;
                                        }
                                        *is_reconnecting.lock().await = false;
                                    } else {
                                        handler.handle_state_change(ClientState::Disconnected).await;
                                        break;
                                    }
                                }
                            }
                        }
                        ClientState::Disconnected => break,
                        _ => {}
                    }
                }
            }
        });
    }
}

pub struct FlareClientBuilder<S, M>
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    ws_url: Option<String>,
    quic_addr: Option<String>,
    quic_server_name: Option<String>,
    quic_cert_path: Option<String>,
    quic_is_test: bool,
    client_config: Option<ClientConfig>,
    handler: Option<ClientMessageHandler<S, M>>,
    protocol: Protocol,
}

impl<S, M> FlareClientBuilder<S, M>
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            ws_url: None,
            quic_addr: None,
            quic_server_name: None,
            quic_cert_path: None,
            quic_is_test: false,
            client_config: None,
            handler: None,
            protocol: Protocol::Auto,
        }
    }

    pub fn ws_url(mut self, url: impl Into<String>) -> Self {
        self.ws_url = Some(url.into());
        self
    }

    pub fn quic_addr(mut self, addr: impl Into<String>) -> Self {
        self.quic_addr = Some(addr.into());
        self
    }

    pub fn quic_server_name(mut self, name: impl Into<String>) -> Self {
        self.quic_server_name = Some(name.into());
        self
    }

    pub fn quic_cert_path(mut self, path: impl Into<String>) -> Self {
        self.quic_cert_path = Some(path.into());
        self
    }

    pub fn quic_is_test(mut self, is_test: bool) -> Self {
        self.quic_is_test = is_test;
        self
    }

    pub fn client_config(mut self, config: ClientConfig) -> Self {
        self.client_config = Some(config);
        self
    }

    pub fn handler(mut self, handler: ClientMessageHandler<S, M>) -> Self {
        self.handler = Some(handler);
        self
    }

    pub fn protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        self
    }

    pub fn use_websocket(mut self) -> Self {
        self.protocol = Protocol::WebSocket;
        self
    }

    pub fn use_quic(mut self) -> Self {
        self.protocol = Protocol::Quic;
        self
    }

    pub fn build(self) -> Result<FlareClient<S, M>> {
        let handler = self.handler.ok_or_else(|| anyhow::anyhow!("Handler is required"))?;
        let client_config = self.client_config.unwrap_or_default();
        
        // 根据选择的协议验证必要参数
        match self.protocol {
            Protocol::Auto | Protocol::WebSocket => {
                if self.ws_url.is_none() {
                    return Err(anyhow::anyhow!("WebSocket URL is required").into());
                }
            }
            Protocol::Quic => {
                if self.quic_addr.is_none() || self.quic_server_name.is_none() || 
                   self.quic_cert_path.is_none() {
                    return Err(anyhow::anyhow!("QUIC configuration is incomplete").into());
                }
            }
        }
        
        Ok(FlareClient::new(
            self.ws_url.unwrap_or_default(),
            self.quic_addr.unwrap_or_default(),
            self.quic_server_name.unwrap_or_default(),
            self.quic_cert_path.unwrap_or_default(),
            self.quic_is_test,
            client_config,
            handler,
            self.protocol,
        ))
    }
}

impl<S, M> Default for FlareClientBuilder<S, M>
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}