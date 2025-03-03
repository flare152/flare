use flare_core::context::{AppContext, AppContextBuilder};
use flare_core::error::{FlareErr, Result};
use crate::connections::Connection;
use crate::server::handlers::{CommandHandler, ServerMessageHandler};
use log::{debug, error, info, warn};
use prost::Message;
use flare_core::flare_net::net::LoginResp;
use flare_core::flare_net::net::{Command, Message as ProtoMessage, Platform, ResCode, Response};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use crate::server::auth_handler::AuthHandler;
use crate::server::server_handler::ServerHandler;
use crate::server::sys_handler::SystemHandler;

use super::auth_handler::DefAuthHandler;
use super::server_handler::DefServerHandler;
use super::sys_handler::DefSystemHandler;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(90);

/// 连接信息
#[derive(Clone)]
pub struct ConnectionInfo {
    protocol: String,
    conn_id: String,
    user_id: String,
    platform: Platform,
    language: Option<String>,
    client_id : String,
    remote_addr: String,
    connected_at: chrono::DateTime<chrono::Utc>,
    last_heartbeat: Arc<Mutex<chrono::DateTime<chrono::Utc>>>,
    conn: Arc<Box<dyn Connection>>,
}

impl ConnectionInfo {
    pub fn new(
        conn: Box<dyn Connection>,
        user_id: String,
        platform: Platform,
        client_id: String,
        remote_addr: String,
        protocol: String,
    ) -> Self {
        Self {
            conn_id: conn.id().to_string(),
            user_id,
            platform,
            language: None,
            client_id,
            remote_addr,
            protocol,
            connected_at: chrono::Utc::now(),
            last_heartbeat: Arc::new(Mutex::new(chrono::Utc::now())),
            conn: Arc::new(conn),
        }
    }

    pub async fn send(&self, msg: ProtoMessage) -> Result<()> {
        self.conn.send(msg).await
    }

    pub async fn receive(&self) -> Result<ProtoMessage> {
        self.conn.receive().await
    }

    pub async fn close(&self) -> Result<()> {
        self.conn.close().await
    }
    pub fn get_conn_id(&self) -> String {
        self.conn_id.clone()
    }
    pub fn get_protocol(&self) -> String {
        self.protocol.clone()
    }
    pub fn get_connection_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.connected_at.clone()
    }
}

pub struct Server<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    handler: Arc<ServerMessageHandler<S, A, Y>>,
    connections: Arc<Mutex<HashMap<String, ConnectionInfo>>>, // conn_id -> ConnectionInfo
    user_connections: Arc<Mutex<HashMap<String, Vec<String>>>>, // user_id -> Vec<conn_id>
}

impl<S, A, Y> Server<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    pub fn new(handler: ServerMessageHandler<S, A, Y>) -> Self {
        let server = Self {
            handler: Arc::new(handler),
            connections: Arc::new(Mutex::new(HashMap::new())),
            user_connections: Arc::new(Mutex::new(HashMap::new())),
        };

        // 启动心跳检测
        let connections = server.connections.clone();
        tokio::spawn(async move {
            let mut interval = interval(HEARTBEAT_INTERVAL);
            loop {
                interval.tick().await;
                Self::check_connections(connections.clone()).await;
            }
        });

        server
    }

    /// 添加新连接
    pub async fn add_connection(&self, conn: Box<dyn Connection>) {
        let conn_id = conn.id().to_string();
        let remote_addr = conn.remote_addr().to_string();
        info!("New connection from {}: {}", remote_addr, conn_id);
        // 等待认证消息
        match self.wait_for_auth(&conn).await {
            Ok(login_resp) => {
                let info = ConnectionInfo::new(
                    conn.clone_box(),
                    login_resp.user_id.clone(),
                    conn.platform(),
                    conn.id().to_string(),
                    conn.remote_addr().to_string(),
                    conn.protocol().to_string(),
                );

                // 保存连接信息
                {
                    let mut conns = self.connections.lock().await;
                    conns.insert(conn_id.clone(), info.clone());
                    
                    // 处理新连接
                    let ctx = match self.build_context(
                        AppContextBuilder::new()
                            .user_id(login_resp.user_id.clone())
                            .remote_addr(info.remote_addr.clone())
                            .platform(info.platform as i32)
                            .client_id(info.client_id.clone()),
                        info.conn_id.clone(),
                        info.client_id.clone(),
                    ).await {
                        Some(ctx) => ctx,
                        None => {
                            error!("Failed to build context for new connection");
                            // 发送错误响应
                            if let Err(e) = conn.send(ProtoMessage {
                                command: Command::ServerResponse as i32,
                                data: Response {
                                    code: ResCode::InvalidParams as i32,
                                    message: "Failed to initialize connection context".into(),
                                    data: Vec::new(),
                                }.encode_to_vec(),
                                ..Default::default()
                            }).await {
                                error!("Failed to send error response: {}", e);
                            }
                            // 关闭连接
                            if let Err(e) = conn.close().await {
                                error!("Failed to close connection: {}", e);
                            }
                            return;
                        }
                    };
                        
                    if let Err(e) = self.handler.handle_new_connection(&ctx, &info).await {
                        error!("Failed to handle new connection: {}", e);
                        // 发送错误响应
                        if let Err(send_err) = conn.send(ProtoMessage {
                            command: Command::ServerResponse as i32,
                            data: Response {
                                code: ResCode::InternalError as i32,
                                message: format!("Failed to initialize connection: {}", e),
                                data: Vec::new(),
                            }.encode_to_vec(),
                            ..Default::default()
                        }).await {
                            error!("Failed to send error response: {}", send_err);
                        }
                        // 关闭连接
                        if let Err(close_err) = conn.close().await {
                            error!("Failed to close connection: {}", close_err);
                        }
                        return;
                    }
                }

                // 更新用户连接映射
                {
                    let mut user_conns = self.user_connections.lock().await;
                    user_conns.entry(login_resp.user_id)
                        .or_insert_with(Vec::new)
                        .push(conn_id.clone());
                }

                // 启动消息处理
                self.handle_connection(info).await;
            }
            Err(e) => {
                error!("Authentication failed for {}: {}", remote_addr, e);
                
                // 发送认证失败响应
                if let Err(send_err) = conn.send(ProtoMessage {
                    command: Command::ServerResponse as i32,
                    data: Response {
                        code: e.code() as i32,
                        message: e.to_string(),
                        data: Vec::new(),
                    }.encode_to_vec(),
                    ..Default::default()
                }).await {
                    error!("Failed to send auth error response: {}", send_err);
                }
                
                // 关闭连接
                if let Err(close_err) = conn.close().await {
                    error!("Failed to close connection after auth failure: {}", close_err);
                }
            }
        }
    }

    /// 构建应用上下文
    async fn build_context(
        &self,
        builder: AppContextBuilder,
        conn_id: String,
        client_msg_id: String,
    ) -> Option<AppContext> {
        match builder.build() {
            Ok(ctx) => Some(ctx),
            Err(e) => {
                error!("Failed to build context: {}", e);
                if let Err(send_err) = self.send_response(
                    conn_id,
                    client_msg_id,
                    Response {
                        code: ResCode::InvalidParams as i32,
                        message: "Invalid context parameters".into(),
                        data: Vec::new(),
                    },
                ).await {
                    error!("Failed to send error response: {}", send_err);
                }
                None
            }
        }
    }

    /// 等待认证消息
    async fn wait_for_auth(&self, conn: &Box<dyn Connection>) -> Result<LoginResp> {
        let timeout = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                msg = conn.receive() => {
                    match msg {
                        Ok(msg) => {
                            match Command::try_from(msg.command) {
                                Ok(Command::Ping) => {
                                    // 处理心跳消息
                                    debug!("Received ping during auth, sending pong");
                                    if let Err(e) = conn.send(ProtoMessage {
                                        command: Command::Pong as i32,
                                        ..Default::default()
                                    }).await {
                                        error!("Failed to send pong: {}", e);
                                    }
                                    continue;
                                }
                                Ok(Command::Pong) => {
                                    // 处理心跳响应
                                    debug!("Received pong during auth, ignoring");
                                    continue;
                                }
                                Ok(Command::Login) => {
                                    // 处理登录请求
                                    let ctx = AppContextBuilder::new()
                                        .remote_addr(conn.remote_addr().to_string())
                                        .command(Some(Command::Login))
                                        .data(msg.data)
                                        .build()?;

                                    match self.handler.handle_auth(&ctx).await {
                                        Ok(response) => {
                                            // 发送登录响应
                                            conn.send(ProtoMessage {
                                                command: Command::ServerResponse as i32,
                                                data: response.data.clone(),
                                                client_id: msg.client_id,
                                                ..Default::default()
                                            }).await?;

                                            // 解析登录响应
                                            if response.code == ResCode::Success as i32 {
                                                if let Ok(login_resp) = LoginResp::decode(&response.data[..]) {
                                                    return Ok(login_resp);
                                                }
                                            }
                                            return Err(FlareErr::AuthError(response.message));
                                        }
                                        Err(e) => return Err(e),
                                    }
                                }
                                _ => {
                                    warn!("Unexpected command during auth: {:?}", msg.command);
                                    continue;
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                _ = &mut timeout => {
                    return Err(FlareErr::AuthError("Authentication timeout".to_string()));
                }
            }
        }
    }

    /// 处理连接
    async fn handle_connection(&self, info: ConnectionInfo) {
        let conn_id = info.conn_id.clone();
        let last_heartbeat = info.last_heartbeat.clone();
        let handler = self.handler.clone();
        let connections = self.connections.clone();
        let info = info.clone();
        let server = Arc::new(ServerHandle {
            handler,
            connections,
        });

        tokio::spawn(async move {
            while let Ok(msg) = info.receive().await {
                debug!("Received message from {}: {:?}", info.remote_addr, msg);
                *last_heartbeat.lock().await = chrono::Utc::now();

                match Command::try_from(msg.command) {
                    Ok(comm) => {
                        if comm == Command::Ping{
                            // 处理心跳消息
                            debug!("Received ping during auth, sending pong");
                            if let Err(e) = server.send_pong(info.conn_id.clone()).await{
                                debug!("Failed to send pong: {}", e)
                            }
                            continue;
                        }
                        if comm ==Command::Pong {
                            // 处理心跳响应
                            debug!("Received pong during auth, ignoring");
                            continue;
                        }
                        let ctx = match server.build_context(
                            AppContextBuilder::new()
                                .user_id(info.user_id.clone())
                                .remote_addr(info.remote_addr.clone())
                                .command(Some(comm))
                                .platform(info.platform as i32)
                                .data(msg.data.clone())
                                .with_language(info.language.clone())
                                .client_id(info.client_id.clone()),
                            info.conn_id.clone(),
                            msg.client_id.clone(),
                        ).await {
                            Some(ctx) => ctx,
                            None => break,
                        };

                        // 处理消息
                        match server.handler.handle_command(&ctx).await {
                            Ok(response) => {
                                if let Err(e) = server.send_response(info.conn_id.clone(), msg.client_id, response).await {
                                    error!("Failed to send response: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Message handling error: {}", e);
                                if let Err(e) = server.send_response(info.conn_id.clone(), msg.client_id, Response {
                                    code: ResCode::InternalError as i32,
                                    message: e.to_string(),
                                    data: Vec::new(),
                                }).await {
                                    error!("Failed to send error response: {}", e);
                                }
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid command: {}", e);
                        if let Err(e) = server.send_response(info.conn_id.clone(), msg.client_id, Response {
                            code: ResCode::InvalidCommand as i32,
                            message: "Invalid command".into(),
                            data: Vec::new(),
                        }).await {
                            error!("Failed to send invalid command response: {}", e);
                        }
                        break;
                    }
                }
            }

            let mut conns = server.connections.lock().await;
            conns.remove(&conn_id);
            info!("Connection closed: {}", conn_id);
        });
    }

    /// 检查连接状态
    async fn check_connections(connections: Arc<Mutex<HashMap<String, ConnectionInfo>>>) {
        let mut conns = connections.lock().await;
        let now = chrono::Utc::now();
        
        conns.retain(|conn_id, info| {
            if let Ok(last) = info.last_heartbeat.try_lock() {
                if now.signed_duration_since(*last) > chrono::Duration::seconds(CONNECTION_TIMEOUT.as_secs() as i64) {
                    warn!("Connection {} timed out", conn_id);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
    }

    /// 向用户发送消息
    pub async fn send_to_user(&self, user_id: &str, msg: ProtoMessage) -> Result<()> {
        let user_conns = self.user_connections.lock().await;
        if let Some(conn_ids) = user_conns.get(user_id) {
            let conns = self.connections.lock().await;
            for conn_id in conn_ids {
                if let Some(info) = conns.get(conn_id) {
                    if let Err(e) = info.send(msg.clone()).await {
                        warn!("Failed to send message to {}: {}", conn_id, e);
                    }
                }
            }
        }
        Ok(())
    }


    /// 向所有连接广播消息
    pub async fn broadcast(&self, msg: ProtoMessage) -> Result<()> {
        let conns = self.connections.lock().await;
        for (conn_id, info) in conns.iter() {
            if let Err(e) = info.send(msg.clone()).await {
                warn!("Failed to broadcast to {}: {}", conn_id, e);
            }
        }
        Ok(())
    }

    /// 获取连接信息
    pub async fn get_connection_info(&self, conn_id: &str) -> Option<ConnectionInfo> {
        let conns = self.connections.lock().await;
        conns.get(conn_id).cloned()
    }

    /// 获取用户的所有连接
    pub async fn get_user_connections(&self, user_id: &str) -> Vec<ConnectionInfo> {
        let user_conns = self.user_connections.lock().await;
        let conns = self.connections.lock().await;
        
        user_conns.get(user_id)
            .map(|conn_ids| {
                conn_ids.iter()
                    .filter_map(|id| conns.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_handler_mut(&mut self) -> &mut ServerMessageHandler<S, A, Y> {
        Arc::get_mut(&mut self.handler).unwrap()
    }
    /// 发送响应消息
    pub async fn send_response(&self, conn_id: String,client_msg_id:String, response: Response) -> Result<()> {
        let conn = self.connections.lock().await;
        if let Some(info) = conn.get(conn_id.as_str()) {
            info.send(ProtoMessage {
                command: Command::ServerResponse as i32,
                data: response.encode_to_vec(),
                client_id:client_msg_id,
                ..Default::default()
            }).await
        } else {
            debug!("Connection not found: {}", conn_id);
            Err(FlareErr::ConnectionNotFound)
        }
    }
}

// 新增一个辅助结构体来处理生命周期问题
struct ServerHandle<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    handler: Arc<ServerMessageHandler<S, A, Y>>,
    connections: Arc<Mutex<HashMap<String, ConnectionInfo>>>,
}

impl<S, A, Y> ServerHandle<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    async fn build_context(&self, builder: AppContextBuilder, conn_id: String, client_msg_id: String) -> Option<AppContext> {
        match builder
            .with_conn_id(conn_id)
            .with_client_msg_id(client_msg_id)
            .build() 
        {
            Ok(ctx) => Some(ctx),
            Err(_) => None
        }
    }

    async fn send_response(&self, conn_id: String, client_msg_id: String, response: Response) -> Result<()> {
        let conn = self.connections.lock().await;
        if let Some(info) = conn.get(conn_id.as_str()) {
            info.send(ProtoMessage {
                command: Command::ServerResponse as i32,
                data: response.encode_to_vec(),
                client_id: client_msg_id,
                ..Default::default()
            }).await
        } else {
            debug!("Connection not found: {}", conn_id);
            Err(FlareErr::ConnectionNotFound)
        }
    }
    //发送pong
    async fn send_pong(&self, conn_id: String)->Result<()> {
        let conn = self.connections.lock().await;
        if let Some(info) = conn.get(conn_id.as_str()) {
            info.send(ProtoMessage {
                command: Command::Pong as i32,
                data: "pong".into(),
                ..Default::default()
            }).await
        } else {
            debug!("Connection not found: {}", conn_id);
            Err(FlareErr::ConnectionNotFound)
        }
    }
}

impl Default for Server<DefServerHandler, DefAuthHandler, DefSystemHandler> {
    fn default() -> Self {
        Self::new(ServerMessageHandler::<DefServerHandler, DefAuthHandler, DefSystemHandler>::default())
    }
} 