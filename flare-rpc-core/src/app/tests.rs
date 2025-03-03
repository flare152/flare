// use super::*;
// use crate::discover::{ConsulConfig, ConsulRegistry};
// use std::time::Duration;
// use tonic::{Request, Response, Status};
//
// // 定义 proto 服务
// pub mod echo {
//     tonic::include_proto!("echo");
// }
//
// use echo::echo_server::{Echo, EchoServer};
// use echo::{EchoRequest, EchoResponse};
//
// // 创建一个 Echo 服务
// #[derive(Default)]
// pub struct EchoService {}
//
// #[tonic::async_trait]
// impl Echo for EchoService {
//     async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
//         let message = request.into_inner().message;
//         Ok(Response::new(EchoResponse { message }))
//     }
// }
//
// #[tokio::test]
// async fn test_app_with_consul() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     // 创建 Consul 配置
//     let consul_config = ConsulConfig {
//         addr: "127.0.0.1:8500".to_string(),
//         timeout: Duration::from_secs(3),
//         protocol: "http".to_string(),
//         token: None,
//     };
//
//     // 创建 Consul 注册器
//     let registry = ConsulRegistry::new(consul_config, Duration::from_secs(15)).await?;
//
//     // 创建并配置应用
//     let app = App::builder("echo-service")
//         .version("1.0.0")
//         .tag("test")
//         .meta("env", "test")
//         .register(registry)
//         .build();
//
//     // 创建 Echo 服务
//     let echo_service = EchoService::default();
//     let svc = EchoServer::new(echo_service);
//
//     // 运行应用
//     app.run("127.0.0.1", 8080, |mut server| async move {
//         server.add_service(svc)
//             .serve("127.0.0.1:8080".parse()?)
//             .await?;
//         Ok(())
//     }).await?;
//
//     Ok(())
// }