use im_core::Connection;
use im_core::quic::QuicConnection;
use protobuf_codegen::{Command, Message};
use std::io::{self, Write, BufReader};
use std::net::SocketAddr;
use std::sync::Arc;
use std::fs::File;
use std::path::Path;
use env_logger;
use log::info;
use quinn::{Endpoint, TransportConfig, ServerConfig, ClientConfig};
use rustls::{RootCertStore, ServerConfig as RustlsServerConfig, ClientConfig as RustlsClientConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use im_core::quic::config::{create_client_config, create_server_config};



fn load_certs(path: &str) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let file = File::open(Path::new(path))?;
    let mut reader = BufReader::new(file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .filter_map(|result| result.ok())
        .collect();
    if certs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "no certificates found"));
    }
    Ok(certs)
}

fn load_private_key(path: &str) -> std::io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    
    if let Some(key) = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .filter_map(|result| result.ok())
        .next() 
    {
        return Ok(PrivateKeyDer::Pkcs8(key));
    }
    
    let mut reader = BufReader::new(File::open(path)?);
    if let Some(key) = rustls_pemfile::rsa_private_keys(&mut reader)
        .filter_map(|result| result.ok())
        .next() 
    {
        return Ok(PrivateKeyDer::Pkcs1(key));
    }
    
    Err(io::Error::new(io::ErrorKind::InvalidData, "no private key found"))
}

fn configure_server() -> anyhow::Result<(Endpoint, SocketAddr)> {
    let server_config = create_server_config("certs/cert.pem", "certs/key.pem")?;
    let addr = "127.0.0.1:8443".parse()?;
    let endpoint = Endpoint::server(server_config, addr)?;
    Ok((endpoint, addr))
}

fn configure_client() -> anyhow::Result<Endpoint> {
    let mut client_config = create_client_config("certs/cert.pem")?;

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    Ok(endpoint)
}

async fn run_server() -> anyhow::Result<()> {
    let (endpoint, addr) = configure_server()?;
    info!("Server listening on: {}", addr);

    while let Some(conn) = endpoint.accept().await {
        let remote = conn.remote_address();
        info!("New client connected: {}", remote);
        
        tokio::spawn(async move {
            match conn.await {
                Ok(conn) => {
                    if let Err(e) = handle_connection(conn, remote.to_string()).await {
                        info!("Connection error: {}", e);
                    }
                }
                Err(e) => info!("Connection failed: {}", e),
            }
        });
    }
    Ok(())
}

async fn handle_connection(conn: quinn::Connection, remote_addr: String) -> anyhow::Result<()> {
    info!("Handling new connection from: {}", remote_addr);
    
    // 等待客户端打开双向流
    let (mut send, mut recv) = conn.accept_bi().await
        .map_err(|e| anyhow::anyhow!("Failed to accept stream: {}", e))?;
    
    // 读取客户端的初始消息
    let mut hello = [0u8; 5];
    recv.read_exact(&mut hello).await
        .map_err(|e| anyhow::anyhow!("Failed to read hello: {}", e))?;
    info!("Received hello: {}", String::from_utf8_lossy(&hello));

    // 创建 QUIC 连接
    let conn = QuicConnection::with_streams(conn, send, recv, remote_addr).await?;
    info!("Connection established");

    while let Ok(msg) = conn.receive().await {
        info!("Received message: {:?}", msg);
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

    Ok(())
}

async fn run_client() -> anyhow::Result<()> {
    let addr = "127.0.0.1:8443".parse()?;
    let endpoint = configure_client()?;

    let conn: quinn::Connection = endpoint
        .connect(addr, "hugo.im.quic.cn")?
        .await?;
    
    let client = Arc::new(QuicConnection::new(conn, addr.to_string()).await?);
    info!("Connected to: {}", addr);

    // 创建一个任务来处理接收消息
    let client_recv = client.clone();
    let receive_task = tokio::spawn(async move {
        while let Ok(msg) = client_recv.receive().await {
            println!("收到消息: {:?}", msg);
            if msg.command == Command::ServerPushMsg as i32 {
                println!("收到服务器消息: {}", String::from_utf8_lossy(&msg.data));
            }
        }
    });

    // 主循环处理用户输入
    loop {
        print!("> ");
        io::stdout().flush()?;

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
        info!("发送消息: {}", input);
    }

    client.close().await?;
    receive_task.abort();
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <server|client>", args[0]);
        return Ok(());
    }

    match args[1].as_str() {
        "server" => run_server().await,
        "client" => run_client().await,
        _ => {
            println!("Invalid argument. Use 'server' or 'client'");
            Ok(())
        }
    }
} 