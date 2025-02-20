use async_trait::async_trait;
use futures::StreamExt;
use im_core::client::client::{Client, ClientState};
use im_core::client::config::ClientConfig;
use im_core::client::message_handler::MessageHandler;
use im_core::client::sys_handler::ClientSystemHandler;
use im_core::common::ctx::AppContext;
use im_core::common::error::error::Result;
use im_core::connections::{Connection, WsConnection};
use im_core::server::auth_handler::AuthHandler;
use im_core::server::handlers::ServerMessageHandler;
use im_core::server::server::Server;
use im_core::server::server_handler::ServerHandler;
use im_core::server::sys_handler::SystemHandler;
use log::{debug, info};
use protobuf_codegen::{Command, Message as ProtoMessage, Platform, Response};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, connect_async};

// 自定义客户端消息处理器
struct ChatClientHandler;

#[async_trait]
impl MessageHandler for ChatClientHandler {
    async fn on_message(&self, msg: Vec<u8>) {
        println!("\r收到消息: {}", String::from_utf8_lossy(&msg));
        print!("> ");
        io::stdout().flush().unwrap();
    }

    async fn on_custom_message(&self, msg: Vec<u8>) {
        println!("\r收到自定义消息: {}", String::from_utf8_lossy(&msg));
    }

    async fn on_notice_message(&self, msg: Vec<u8>) {
        println!("\r收到通知: {}", String::from_utf8_lossy(&msg));
    }

    async fn on_response(&self, msg: &Response) {
        println!("\r收到响应: {:?}", msg);
    }

    async fn on_ack_message(&self, msg: Vec<u8>) {
        println!("\r收到确认: {}", String::from_utf8_lossy(&msg));
    }

    async fn on_data(&self, data: Vec<u8>) {
        println!("\r收到数据: {}", String::from_utf8_lossy(&data));
    }
}

// 自定义客户端系统处理器
struct ChatClientSysHandler;

#[async_trait]
impl ClientSystemHandler for ChatClientSysHandler {
    async fn login_out(&self) -> Result<()> {
        println!("系统通知: 已退出登录");
        Ok(())
    }

    async fn set_background(&self) -> Result<()> {
        println!("系统通知: 已切换到后台");
        Ok(())
    }

    async fn set_language(&self) -> Result<()> {
        println!("系统通知: 语言已更新");
        Ok(())
    }

    async fn kick_online(&self) -> Result<()> {
        println!("系统通知: 您已被踢下线");
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        println!("系统通知: 连接已关闭");
        Ok(())
    }

    async fn on_state_change(&self, state: ClientState) {
        println!("系统通知: 连接状态变更为 {:?}", state);
    }
}

// 自定义服务端消息处理器
struct ChatServerHandler;

#[async_trait]
impl ServerHandler for ChatServerHandler {
    async fn handle_send_message(&self, ctx: &AppContext) -> Result<Response> {
        let msg = ctx.string_data()?;
        let prefix = "你好, ".to_string();
        let content = format!("{}{}", prefix, msg);
        
        Ok(Response {
            code: 0,
            message: "消息已发送".into(),
            data: content.into_bytes(),
        })
    }

    async fn handle_pull_message(&self, ctx: &AppContext) -> Result<Response> {
        debug!("处理拉取消息请求");
        Ok(Response::default())
    }

    async fn handle_request(&self, ctx: &AppContext) -> Result<Response> {
        debug!("处理数据请求");
        Ok(Response::default())
    }

    async fn handle_ack(&self, ctx: &AppContext) -> Result<Response> {
        debug!("处理消息确认");
        Ok(Response::default())
    }
}

async fn run_server() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("聊天服务器监听端口: {}", addr);

    let server = Server::new(ServerMessageHandler::default());

    while let Ok((stream, addr)) = listener.accept().await {
        info!("新客户端连接: {}", addr);
        let ws_stream = accept_async(stream).await.unwrap();
        let conn = Box::new(WsConnection::new(ws_stream, addr.to_string()));
        server.add_connection(conn).await;
    }
}

async fn run_client() {
    let url = "ws://127.0.0.1:8080";
    let (ws_stream, _) = connect_async(url).await.unwrap();
    
    let config = ClientConfig::default();
    let connector = || Box::pin(async { 
        Ok(Box::new(WsConnection::new(ws_stream, "client".to_string())) as Box<dyn Connection>)
    });

    let client = Client::new(connector, config);
    info!("已连接到服务器: {}", url);

    // 主循环处理用户输入
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "quit" {
            break;
        }

        let msg = ProtoMessage {
            command: Command::ClientSendMessage as i32,
            data: input.as_bytes().to_vec(),
            ..Default::default()
        };

        if let Err(e) = client.send(msg).await {
            println!("发送失败: {}", e);
            break;
        }
    }

    client.close().await.unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("用法: {} <server|client>", args[0]);
        return;
    }

    match args[1].as_str() {
        "server" => run_server().await,
        "client" => run_client().await,
        _ => println!("无效参数。请使用 'server' 或 'client'"),
    }
}