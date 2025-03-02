use rpc_core::discover::{ConsulConfig, ConsulDiscover, ConsulRegistry, LoadBalanceStrategy, RpcDiscovery};
use rpc_core::client::call_rpc;
use flare::context::AppContextBuilder;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tonic::transport::Channel;

// 包含生成的 proto 代码
tonic::include_proto!("echo");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 初始化日志
    env_logger::init();
    let config = ConsulConfig {
        addr: "127.0.0.1:8500".to_string(),
        timeout: std::time::Duration::from_secs(10),
        protocol: "http".to_string(),
        token: None,
    };
    // 等待 Consul 注册中心启动
    let discovery = ConsulDiscover::new(config.clone(), LoadBalanceStrategy::RoundRobin);
    discovery.start_watch().await;
    // 从注册中心获取服务地址
    let service = discovery.discover("echo-service").await?;
    let addr = format!("http://{}:{}", service.address, service.port);

    // 创建客户端连接
    let channel = Channel::from_shared(addr)
        .unwrap()
        .connect()
        .await?;

    let client = echo_client::EchoClient::new(channel);

    // 创建上下文
    let ctx = AppContextBuilder::new()
        .remote_addr("127.0.0.1:12345".to_string())
        .user_id("test-user-001".to_string())
        .platform(1)
        .client_id("test-client-001".to_string())
        .with_language(Some("zh-CN".to_string()))
        .with_conn_id("test-conn-001".to_string())
        .with_client_msg_id("test-msg-001".to_string())
        .values(Arc::new(Mutex::new({
            let mut values = HashMap::new();
            values.insert("request_id".to_string(), "test-123".to_string());
            values.insert("trace_id".to_string(), "trace-001".to_string());
            values
        })))
        .build()
        .expect("Failed to build AppContext");

    // 准备请求参数
    let request = EchoRequest {
        message: "Hello from client!".to_string(),
    };

    // 发送请求
    let response = call_rpc(
        ctx,
        client,
        request,
        |mut client, request| Box::pin(async move {
            client.echo(request).await
        })
    ).await?;

    println!("Response: {}", response.into_inner().message);
    Ok(())
}