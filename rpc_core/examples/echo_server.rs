use rpc_core::app::{App, AppBuilder};
use rpc_core::discover::{ConsulConfig, ConsulRegistry};
use tonic::{Request, Response, Status};
use std::error::Error;
use std::time::Duration;

// 包含生成的 proto 代码
tonic::include_proto!("echo");

struct EchoService;

#[tonic::async_trait]
impl echo_server::Echo for EchoService {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        // 从请求的 metadata 中获取上下文信息
        let metadata = request.metadata();
        let ctx = rpc_core::interceptor::build_context_from_metadata(metadata)?;
        
        // 获取请求消息
        let msg = request.into_inner().message;
        
        // 构造响应，加入上下文信息
        let response = format!("Echo: {} (from user: {:?})", msg, ctx.user_id());
        Ok(Response::new(EchoResponse { message: response }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 初始化日志
    env_logger::init();

    // 创建 Consul 配置
    let consul_config = ConsulConfig {
        addr: "127.0.0.1:8500".to_string(),
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建 Consul 注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(15)).await?;


    // 创建并配置应用
    let app = AppBuilder::new("echo-service")
        .version("1.0.0")
        .tag("rpc")
        .tag("echo")
        .meta("protocol", "grpc")
        .weight(10)
        .register(registry)
        .build();

    // 创建 Echo 服务
    let echo_service = EchoService;
    let echo_server = echo_server::EchoServer::new(echo_service);

    // 运行服务器
    app.run("127.0.0.1", 50051, |mut server| async move {
        server.add_service(echo_server)
            .serve("127.0.0.1:50051".parse()?)
            .await
            .map_err(|e| e.into())
    }).await?;

    Ok(())
} 