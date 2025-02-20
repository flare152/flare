use im_core::client::client::{Client, ClientState};
use im_core::client::config::ClientConfig;
use im_core::client::message_handler::MessageHandler;
use im_core::common::error::error::{Result, FlareErr};
use im_core::connections::connection::Connection;
use im_core::connections::ws::conn::WsConnection;
use protobuf_codegen::{Command, Message};
use tokio_tungstenite::connect_async;
use std::io::{self, Write};
use log::{info, error};
use async_trait::async_trait;
use std::sync::Arc;
use anyhow::anyhow;
use std::time::{Duration, Instant};

struct ChatHandler;

#[async_trait]
impl MessageHandler for ChatHandler {
    async fn on_message(&self, msg: Message) -> Result<()> {
        if msg.command == Command::ServerPushMsg as i32 {
            println!("\r{}", String::from_utf8_lossy(&msg.data));
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Ok(())
    }

    async fn on_state_change(&self, state: ClientState) {
        info!("Connection state changed to: {}", state);
    }
}

async fn create_connection() -> Result<Box<dyn im_core::Connection>> {
    let url = url::Url::parse("ws://127.0.0.1:8080")
        .map_err(|e| FlareErr::ConnectionError(format!("Invalid URL: {}", e)))?;
    
    let (ws_stream, _) = connect_async(url).await
        .map_err(|e| FlareErr::ConnectionError(format!("WebSocket connection failed: {}", e)))?;
    
    Ok(Box::new(WsConnection::new(ws_stream, "localhost".to_string())))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("Enter your username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
    let username = username.trim().to_string();

    // 创建客户端配置
    let mut config = ClientConfig::default();
    config.auth_token = username.clone();

    // 创建客户端
    let client = Arc::new(Client::new(
        || Box::pin(create_connection()),
        ChatHandler,
        config,
    ));

    // 连接到服务器
    client.connect().await?;
    
    // 等待连接就绪，最多等待 5 秒
    client.wait_ready(Duration::from_secs(5)).await?;
    
    // 打印连接状态
    let status = client.connection_status().await;
    info!("Connection status: {}", status);

    // 创建一个克隆用于消息发送
    let client_sender = client.clone();

    // 处理用户输入
    tokio::spawn(async move {
        loop {
            // 检查连接状态
            if !client_sender.is_connected().await {
                error!("Connection lost, attempting to reconnect...");
                if let Err(e) = client_sender.reconnect().await {
                    error!("Reconnection failed: {}", e);
                    break;
                }
            }

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

            if input == "/quit" {
                break;
            }

            // 处理加入群组命令
            if input.starts_with("/join ") {
                let group = input.trim_start_matches("/join ").trim();
                let msg = Message {
                    command: 2,
                    data: group.as_bytes().to_vec(),
                    ..Default::default()
                };
                if let Err(e) = client_sender.send(msg).await {
                    error!("Failed to send join group message: {}", e);
                }
                continue;
            }

            // 发送聊天消息
            let msg = Message {
                command: Command::ClientSendMessage as i32,
                data: input.as_bytes().to_vec(),
                ..Default::default()
            };

            if let Err(e) = client_sender.send(msg).await {
                error!("Failed to send message: {}", e);
                break;
            }
        }

        let _ = client_sender.close().await;
    });

    // 等待客户端关闭
    tokio::signal::ctrl_c().await.map_err(|e| anyhow!("Failed to handle Ctrl-C: {}", e))?;
    client.close().await?;

    Ok(())
} 