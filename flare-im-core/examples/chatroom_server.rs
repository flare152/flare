use anyhow::anyhow;
use async_trait::async_trait;
use flare_core::context::AppContext;
use flare_core::error::Result;
use flare_im_core::connections::quic_conf::create_server_config;
use flare_im_core::server::auth_handler::{AuthCommandHandler, AuthHandler, DefAuthHandler};
use flare_im_core::server::handlers::ServerMessageHandler;
use flare_im_core::server::server_handler::{DefServerHandler, ServerCommandHandler, ServerHandler};
use flare_im_core::server::sys_handler::{DefSystemHandler, SystemCommandHandler, SystemHandler};
use flare_im_core::telecom::FlareServer;
use log::{error, info};
use prost::Message;
use flare_core::flare_net::net::{Command, Message as ProtoMessage, ResCode, Response};

// 聊天室消息处理器
struct ChatHandler;

impl ChatHandler {
    pub fn new() -> Self {
        Self{}
    }
}

#[async_trait]
impl ServerHandler for ChatHandler {
    async fn handle_send_message(&self, ctx: &AppContext) -> Result<Response> {
        let mut response = Response::default();
        let msg = ctx.data();
        if let Ok(content) = String::from_utf8(msg.to_vec()) {
            let modified_content = format!("hello {}", content);
            let broadcast_msg = ProtoMessage {
                command: Command::ServerPushMsg as i32,
                data: modified_content.clone().into_bytes(),
                ..Default::default()
            };

            response.code = ResCode::Success as i32;
            response.message = "Message sent".to_string();
            response.data = broadcast_msg.encode_to_vec();
        }
        Ok(response)
    }

    async fn handle_pull_message(&self, _ctx: &AppContext) -> Result<Response> {
        Ok(Response::default())
    }

    async fn handle_request(&self, _ctx: &AppContext) -> Result<Response> {
        Ok(Response::default())
    }

    async fn handle_ack(&self, _ctx: &AppContext) -> Result<Response> {
        Ok(Response::default())
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // 创建服务器处理器，使用泛型参数
    let handler = ServerMessageHandler::<ChatHandler, DefAuthHandler, DefSystemHandler>::new(
        AuthCommandHandler::new(DefAuthHandler::new()),
        ServerCommandHandler::new(ChatHandler::new()),
        SystemCommandHandler::new(DefSystemHandler::new())
    );

    // 创建并配置服务器
    let server = FlareServer::builder()
        .ws_addr("127.0.0.1:8080")
        .quic_addr("127.0.0.1:8081")
        .quic_server_name("hugo.im.quic.cn")
        .quic_cert_path("certs/cert.pem")
        .quic_key_path("certs/key.pem")
        .handler(handler)
        .build()?;

    info!("Chat room server starting...");
    
    // 运行服务器
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
    }

    Ok(())
} 