use im_core::ws::WsConnection;
use im_core::{Connection, ConnectionState};
use protobuf_codegen::{Command, Message, Platform};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, connect_async};
use env_logger;

async fn setup_ws_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(handle_connection(stream));
        }
    });

    (format!("ws://{}", addr), handle)
}

async fn handle_connection(stream: TcpStream) {
    let ws_stream = accept_async(stream).await.unwrap();
    let conn = WsConnection::new(ws_stream, "127.0.0.1:0".to_string());

    // 等待客户端消息并回复
    while let Ok(msg) = conn.receive().await {
        if msg.command == Command::Ping as i32 {
            let response = Message::default()
                .with_command(Command::Pong as i32)
                .with_data(b"pong".to_vec());
            conn.send(response).await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_ws_connection() {
    env_logger::init();
    
    // 启动服务器
    let (addr, _handle) = setup_ws_server().await;

    // 创建客户端连接
    let (ws_stream, _) = connect_async(addr).await.unwrap();
    let client = WsConnection::new(ws_stream, "client".to_string());

    // 检查初始状态
    assert_eq!(client.state(), ConnectionState::Connected);
    assert!(!client.is_authenticated());
    assert_eq!(client.platform(), Platform::Unknown);

    // 发送 ping 消息
    let ping = Message::default()
        .with_command(Command::Ping as i32)
        .with_data(b"ping".to_vec());
    client.send(ping).await.unwrap();

    // 等待并验证 pong 响应
    tokio::time::sleep(Duration::from_millis(100)).await;
    let response = client.receive().await.unwrap();
    assert_eq!(response.command, Command::Pong as i32);
    assert_eq!(response.data, b"pong");

    // 关闭连接
    client.close().await.unwrap();
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

// 为 Message 添加一些辅助方法
trait MessageExt {
    fn with_command(self, cmd: i32) -> Self;
    fn with_data(self, data: Vec<u8>) -> Self;
}

impl MessageExt for Message {
    fn with_command(mut self, cmd: i32) -> Self {
        self.command = cmd;
        self
    }

    fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }
} 