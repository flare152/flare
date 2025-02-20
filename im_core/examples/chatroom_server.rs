use anyhow::anyhow;
use async_trait::async_trait;
use im_core::common::ctx::AppContext;
use im_core::common::error::error::Result;
use im_core::connections::WsConnection;
use im_core::server::auth_handler::AuthHandler;
use im_core::server::handlers::ServerMessageHandler;
use im_core::server::server::Server;
use im_core::server::server_handler::ServerHandler;
use im_core::server::sys_handler::SystemHandler;
use log::{error, info};
use protobuf_codegen::{Command, Message as ProtoMessage, Response};

// 聊天室消息处理器
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // 创建服务器
    let  handler = ServerMessageHandler::default();
    let server = Server::new(handler);

    // 启动 WebSocket 服务器
    let addr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| anyhow!("Failed to bind: {}", e))?;
    info!("Chat room server listening on {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(stream).await.map_err(|e| anyhow!("WebSocket error: {}", e))?;
        let conn = Box::new(WsConnection::new(ws_stream, addr.to_string()));
        server.add_connection(conn).await;
    }

    Ok(())
} 