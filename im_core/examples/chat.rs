use futures::StreamExt;
use im_core::{Connection, ConnectionState};
use im_core::ws::WsConnection;
use protobuf_codegen::{Command, Message};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio_tungstenite::{accept_async, connect_async};
use env_logger;
use log::info;

async fn run_server() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Server listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New client connected: {}", addr);
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: TcpStream) {
    let ws_stream = accept_async(stream).await.unwrap();
    let conn = Arc::new(WsConnection::new(ws_stream, "server".to_string()));

    while let Ok(msg) = conn.receive().await {
        if msg.command == Command::ClientSendMessage as i32 {
            let mut response = Message::default();
            response.command = Command::ServerPushMsg as i32;
            
            let prefix = "你好,".to_string();
            let content = format!("{}{}", prefix, String::from_utf8_lossy(&msg.data));
            response.data = content.into_bytes();

            if let Err(e) = conn.send(response).await {
                info!("Failed to send response: {}", e);
                break;
            }
        }
    }
}

async fn run_client() {
    let url = "ws://127.0.0.1:8080";
    let (ws_stream, _) = connect_async(url).await.unwrap();
    let client = Arc::new(WsConnection::new(ws_stream, "client".to_string()));
    info!("Connected to: {}", url);

    // 创建一个任务来处理接收消息
    let client_recv = client.clone();
    let receive_task = tokio::spawn(async move {
        while let Ok(msg) = client_recv.receive().await {
            if msg.command == Command::ServerPushMsg as i32 {
                println!("收到服务器消息: {}", String::from_utf8_lossy(&msg.data));
            }
        }
    });

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

        let mut msg = Message::default();
        msg.command = Command::ClientSendMessage as i32;
        msg.data = input.as_bytes().to_vec();

        if let Err(e) = client.send(msg).await {
            println!("发送失败: {}", e);
            break;
        }
    }

    client.close().await.unwrap();
    receive_task.abort();
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <server|client>", args[0]);
        return;
    }

    match args[1].as_str() {
        "server" => run_server().await,
        "client" => run_client().await,
        _ => println!("Invalid argument. Use 'server' or 'client'"),
    }
} 