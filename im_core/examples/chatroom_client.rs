use anyhow::anyhow;
use im_core::client::config::ClientConfig;
use im_core::client::handlers::ClientMessageHandler;
use im_core::client::message_handler::DefMessageHandler;
use im_core::client::sys_handler::DefClientSystemHandler;
use im_core::telecom::FlareClient;
use log::{error, info};
use protobuf_codegen::{Command, Message};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("Enter your username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .map_err(|e| anyhow!("Failed to read input: {}", e))?;
    let username = username.trim().to_string();

    // 创建客户端配置
    let mut config = ClientConfig::default();
    config.auth_token = username.clone();

    // 创建客户端，使用默认的系统处理器和消息处理器
    let mut client = FlareClient::<DefClientSystemHandler, DefMessageHandler>::builder()
        .ws_url("ws://127.0.0.1:8080")
        .client_config(config)
        .handler(ClientMessageHandler::<DefClientSystemHandler, DefMessageHandler>::default())
        .use_websocket()
        .build()?;

    // 连接到服务器
    client.connect().await?;
    
    // 等待连接就绪
   // client.wait_ready(Duration::from_secs(5)).await?;
    
    // 打印连接状态
    let status = client.get_state().await;
    info!("Connection status: {}", status);

    // 创建一个克隆用于消息发送
    let client_sender = Arc::new(client);

    // 处理用户输入
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

        if input == "/quit" {
            break;
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

    // 关闭客户端
    client_sender.close().await?;

    Ok(())
} 