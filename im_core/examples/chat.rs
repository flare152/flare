use async_trait::async_trait;
use im_core::client::client::Client;
use im_core::client::config::ClientConfig;
use im_core::client::message_handler::MessageHandler;
use flare::context::AppContext;
use flare::error::{FlareErr, Result};
use im_core::connections::{Connection, WsConnection};
use im_core::server::auth_handler::{AuthHandler, DefAuthHandler};
use im_core::server::handlers::ServerMessageHandler;
use im_core::server::server::Server;
use im_core::server::server_handler::{DefServerHandler, ServerHandler};
use im_core::server::sys_handler::{DefSystemHandler, SystemHandler};
use log::{debug, error, info};
use protobuf_codegen::{Command, Message as ProtoMessage, Platform, Response};
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async};
use url;

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

// 自定义服务端消息处理器
struct ChatServerHandler;

#[async_trait]
impl ServerHandler for ChatServerHandler {
    async fn handle_send_message(&self, ctx:  &AppContext) -> Result<Response> {
        let msg = ctx.string_data()?;
        let prefix = "你好, ".to_string();
        let content = format!("{}{}", prefix, msg);
        
        Ok(Response {
            code: 0,
            message: "消息已发送".into(),
            data: content.into_bytes(),
        })
    }

    async fn handle_pull_message(&self, _ctx:  &AppContext) -> Result<Response> {
        debug!("处理拉取消息请求");
        Ok(Response::default())
    }

    async fn handle_request(&self, _ctx:  &AppContext) -> Result<Response> {
        debug!("处理数据请求");
        Ok(Response::default())
    }

    async fn handle_ack(&self, _ctx:  &AppContext) -> Result<Response> {
        debug!("处理消息确认");
        Ok(Response::default())
    }
}

async fn run_server() -> Result<()> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
    info!("聊天服务器监听端口: {}", addr);

    let server = Server::<DefServerHandler, DefAuthHandler, DefSystemHandler>::new(ServerMessageHandler::default());

    while let Ok((stream, addr)) = listener.accept().await {
        info!("新客户端连接: {}", addr);
        let ws_stream = accept_async(stream).await?;
        let conn = Box::new(WsConnection::new(ws_stream, addr.to_string()));
        server.add_connection(conn).await;
    }
    Ok(())
}

async fn run_client() -> Result<()> {
    let url = url::Url::parse("ws://127.0.0.1:8080").unwrap();
    let mut config = ClientConfig::default();
    config.auth_token = "123456".to_string();
    let url_clone = url.clone();
    let connector = move || {
        let url = url.clone();
        Box::pin(async move {
            let (ws_stream, _) = connect_async(url.as_str()).await
                .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
            Ok(Box::new(WsConnection::new(ws_stream, "127.0.0.1:8080".to_string())) as Box<dyn Connection>)
        }) as Pin<Box<dyn Future<Output = Result<Box<dyn Connection>>> + Send + Sync>>
    };

    let client = Client::new(connector, config);
    client.connect().await?;
    info!("已连接到服务器: {}", url_clone);

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

       match client.send(msg).await {
           Ok(r)=> {
               println!("消息已发送: {:?}", r);
           },
           Err(e)=> {
               error!("消息发送失败: {}", e);
           }
        }
    }

    client.close().await.unwrap();
    Ok(())
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
        "server" => if let Err(e) = run_server().await {
            error!("服务器错误: {}", e);
        },
        "client" => if let Err(e) = run_client().await {
            error!("客户端错误: {}", e);
        },
        _ => println!("无效参数。请使用 'server' 或 'client'"),
    }
}