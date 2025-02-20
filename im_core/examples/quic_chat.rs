use im_core::client::client::Client;
use im_core::client::config::ClientConfig;
use im_core::common::error::{FlareErr, Result};
use im_core::connections::quic_conf::{
    create_client_config, create_server_config, init_crypto
};
use im_core::connections::{Connection, QuicConnection};
use log::{debug, error, info};
use protobuf_codegen::{Command, Message as ProtoMessage};
use quinn::Endpoint;
use std::future::Future;
use std::io::{self, Write};
use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;

// 辅助函数：解析地址
fn parse_addr(addr: &str) -> Result<SocketAddr> {
    addr.parse().map_err(|e: AddrParseError| 
        FlareErr::ConnectionError(e.to_string())
    )
}

// 服务器运行逻辑
async fn run_server() -> Result<()> {
    let server_config = create_server_config(
        "/Users/hg/workspace/rust/flare/im_core/certs/cert.pem",
        "/Users/hg/workspace/rust/flare/im_core/certs/key.pem"
    )?;
    
    let addr = parse_addr("127.0.0.1:8443")?;
    let endpoint = Endpoint::server(server_config, addr)
        .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
    
    info!("QUIC 聊天服务器监听端口: {}", addr);

    let server = Arc::new(im_core::server::server::Server::new(
        im_core::server::handlers::ServerMessageHandler::default()
    ));

    while let Some(conn) = endpoint.accept().await {
        let remote = conn.remote_address();
        info!("新客户端连接: {}", remote);

        match conn.await {
            Ok(new_conn) => {
                let quic_conn = QuicConnection::new(new_conn, remote.to_string()).await?;
                server.add_connection(Box::new(quic_conn)).await;
            }
            Err(e) => error!("连接建立失败: {}", e),
        }
    }
    Ok(())
}

// 客户端运行逻辑
async fn run_client() -> Result<()> {
    let client_config = create_client_config("/Users/hg/workspace/rust/flare/im_core/certs/cert.pem",true)
        .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;

    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let mut endpoint = Endpoint::client(bind_addr)
        .map_err(|e| FlareErr::ConnectionError(e.to_string()))?;
    endpoint.set_default_client_config(client_config);

    let server_addr = parse_addr("127.0.0.1:8443")?;
    let mut config = ClientConfig::default();
    config.auth_token = "123456".to_string();

    let connector = move || {
        let endpoint = endpoint.clone();
        let server_addr = server_addr;
        Box::pin(async move {
            match endpoint.connect(server_addr, "hugo.im.quic.cn") {  // 使用证书中的域名
                Ok(connecting) => {
                    match connecting.await {
                        Ok(conn) => {
                            let quic_conn = QuicConnection::new(conn, server_addr.to_string()).await?;
                            Ok(Box::new(quic_conn) as Box<dyn Connection>)
                        }
                        Err(e) => Err(FlareErr::ConnectionError(format!("连接失败: {}", e)))
                    }
                }
                Err(e) => Err(FlareErr::ConnectionError(format!("创建连接失败: {}", e)))
            }
        }) as Pin<Box<dyn Future<Output = Result<Box<dyn Connection>>> + Send + Sync>>
    };

    let client = Client::new(connector, config);
    client.connect().await?;
    info!("已连接到服务器: {}", server_addr);

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
            Ok(_) => debug!("消息已发送"),
            Err(e) => error!("消息发送失败: {}", e),
        }
    }

    client.close().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    // 初始化加密提供程序
    init_crypto().unwrap();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("用法: {} <server|client>", args[0]);
        return Ok(());
    }

    match args[1].as_str() {
        "server" => run_server().await,
        "client" => run_client().await,
        _ => {
            println!("无效参数。请使用 'server' 或 'client'");
            Ok(())
        }
    }
}
