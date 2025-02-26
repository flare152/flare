use super::*;
use crate::discover::{ConsulConfig, ConsulRegistry};
use std::time::Duration;
use volo_grpc::BoxError;
use volo_grpc::server::Server;

#[tokio::test]
async fn test_app_with_consul() -> Result<(), BoxError> {
    // 创建 Consul 配置
    let consul_config = ConsulConfig {
        addr: "127.0.0.1:8500".to_string(),
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(5)).await?;

    // 创建并配置应用
    let app = App::builder("test-service")
        .version("1.0.0")
        .tag("test")
        .meta("env", "test")
        .register(registry)
        .build();

    // 创建 gRPC 服务器
    let server = Server::new();

    // 运行应用
    app.run("127.0.0.1", 8080, |addr| async move {
        server.run(addr).await
    }).await?;

    Ok(())
} 