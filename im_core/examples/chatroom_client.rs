use anyhow::anyhow;
use im_core::client::config::ClientConfig;
use im_core::client::handlers::ClientMessageHandler;
use im_core::client::message_handler::DefMessageHandler;
use im_core::client::sys_handler::DefClientSystemHandler;
use im_core::telecom::{FlareClient, Protocol};
use log::{error, info};
use protobuf_codegen::{Command, Message};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // 获取命令行参数
    let args: Vec<String> = std::env::args().collect();
    let protocol = match args.get(1).map(|s| s.as_str()) {
        Some("ws") => Protocol::WebSocket,
        Some("quic") => Protocol::Quic,
        Some("auto") => Protocol::Auto,
        _ => {
            println!("Usage: {} [ws|quic|auto]", args[0]);
            println!("Defaulting to WebSocket");
            Protocol::WebSocket
        }
    };

    println!("Enter your username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .map_err(|e| anyhow!("Failed to read input: {}", e))?;
    let username = username.trim().to_string();

    // 创建客户端配置
    let mut config = ClientConfig::default();
    config.auth_token = username.clone();

    // 创建客户端构建器
    let mut builder = FlareClient::<DefClientSystemHandler, DefMessageHandler>::builder()
        .client_config(config)
        .handler(ClientMessageHandler::<DefClientSystemHandler, DefMessageHandler>::default());

    // 根据协议设置连接参数
    match protocol {
        Protocol::WebSocket => {
            builder = builder.ws_url("ws://127.0.0.1:8080").use_websocket();
        }
        Protocol::Quic => {
            builder = builder
                .quic_addr("127.0.0.1:8081")
                .quic_server_name("localhost")
                .quic_cert_path("certs/cert.pem")
                .quic_is_test(true)
                .use_quic();
        }
        Protocol::Auto => {
            builder = builder
                .ws_url("ws://127.0.0.1:8080")
                .quic_addr("127.0.0.1:8081")
                .quic_server_name("localhost")
                .quic_cert_path("certs/cert.pem")
                .quic_is_test(true)
                .protocol(Protocol::Auto);
        }
    }

    // 构建并连接客户端
    let mut client = builder.build()?;
    client.connect().await?;
    
    // 等待连接就绪
    client.wait_ready(Duration::from_secs(5)).await?;
    
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