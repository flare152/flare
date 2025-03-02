#[cfg(test)]
mod tests {
    use crate::client::{RpcClient, GrpcClient};
    use crate::discover::consul::{ConsulConfig, ConsulDiscover, ConsulRegistry};
    use crate::discover::{LoadBalanceStrategy, RpcDiscovery, ServiceError, Registration, Registry};
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tonic::{Request, Response, Status};
    use tonic::transport::Channel;
    use crate::client::tests::tests::echo_client::EchoClient;
    use std::collections::HashMap;

    tonic::include_proto!("echo");

    // Echo 服务实现
    #[derive(Debug)]
    struct EchoService;

    #[tonic::async_trait]
    impl echo_server::Echo for EchoService {
        async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
            let msg = request.into_inner().message;
            Ok(Response::new(EchoResponse {
                message: format!("Echo: {}", msg),
            }))
        }
    }

    impl GrpcClient for echo_client::EchoClient<Channel> {
        fn new(channel: Channel) -> Self {
            echo_client::EchoClient::new(channel)
        }
    }

    #[tokio::test]
    async fn test_rpc_client_with_consul() -> Result<(), Box<dyn std::error::Error>> {
        // 1. 启动测试服务器
        let echo_service = EchoService;
        let addr = "127.0.0.1:50051";
        let socket_addr = SocketAddr::from_str(addr)?;
        
        let server_future = tonic::transport::Server::builder()
            .add_service(echo_server::EchoServer::new(echo_service))
            .serve(socket_addr);
        
        tokio::spawn(server_future);
        
        // 等待服务器启动
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 2. 创建 Consul 服务发现并注册服务
        let config = ConsulConfig {
            addr: "127.0.0.1:8500".to_string(),
            timeout: std::time::Duration::from_secs(5),
            protocol: "http".to_string(),
            token: None,
        };
        let registry = ConsulRegistry::new(config.clone(), std::time::Duration::from_secs(30)).await?;
        let discovery = ConsulDiscover::new(config.clone(), LoadBalanceStrategy::RoundRobin);
        
        // 注册服务
        let registration = Registration::new(
            "echo-service-test".to_string(),  // 修改服务名，避免使用下划线
            "echo-service-1".to_string(),
            vec!["test".to_string()],
            "127.0.0.1".to_string(),
            50051,
            1,
            HashMap::new(),
            "1.0.0".to_string(),
        );
        
        registry.register(registration).await?;
        
        // 启动服务发现
        discovery.start_watch().await;
        
        // 等待服务发现完成
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        
        // 3. 创建 RPC 客户端
        let client_factory = RpcClient::<EchoClient<Channel>, _>::new("echo-service-test", discovery.clone());
        
        // 4. 获取客户端并发送请求
        let mut client = client_factory.client().await?;
        let response = client.echo(Request::new(EchoRequest {
            message: "Hello".to_string(),
        })).await?;
        
        // 5. 验证响应
        assert_eq!(response.into_inner().message, "Echo: Hello");
        
        // 6. 注销服务并停止服务发现
        registry.deregister("echo-service-1").await?;
        discovery.stop_watch().await;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_rpc_client_service_not_found() -> Result<(), Box<dyn std::error::Error>> {
        // 创建 Consul 服务发现
        let config = ConsulConfig::default();
        let discovery = ConsulDiscover::new(config, LoadBalanceStrategy::RoundRobin);
        
        // 启动服务发现
        discovery.start_watch().await;
        
        // 创建 RPC 客户端，使用一个不存在的服务名
        let client_factory = RpcClient::<echo_client::EchoClient<Channel>, _>::new("non-existent-service", discovery.clone());
        
        // 尝试获取客户端，应该返回 NotFound 错误
        match client_factory.client().await {
            Err(ServiceError::NotFound(_)) => {
                // 预期的错误
                Ok(())
            },
            Ok(_) => {
                panic!("Expected NotFound error, but got Ok");
            },
            Err(e) => {
                panic!("Expected NotFound error, but got: {:?}", e);
            }
        }
    }
} 