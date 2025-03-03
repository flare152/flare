use crate::interceptor::ctxinterceprot::AppContextConfig;
use flare_core::context::AppContext;
use flare_core::context::AppContextBuilder;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use tower::Layer;
use tower::Service;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub async fn call_rpc<C, P, R>(
    ctx: AppContext,
    client: C,
    params: P,
    rpc_call: impl FnOnce(C, Request<P>) -> Pin<Box<dyn Future<Output = Result<Response<R>, Status>> + Send + 'static>> + Send + 'static,
) -> Result<Response<R>, Status>
where
    C: Clone + Send + 'static,
    P: Send + 'static,
    R: Send + 'static,
{
    // 创建请求并添加上下文元数据
    let mut request = Request::new(params);
    crate::interceptor::ctxinterceprot::build_req_metadata_form_ctx(&ctx, &mut request);
    
    // 执行调用
    rpc_call(client, request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::call::tests::echo_client::EchoClient;
    use crate::client::call::tests::echo_server::{Echo, EchoServer};
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tonic::transport::Channel;
    use crate::discover::ServiceError;

    tonic::include_proto!("echo");

    struct EchoService;

    #[tonic::async_trait]
    impl Echo for EchoService {
        async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
            // 从请求的 metadata 中获取上下文信息
            let metadata = request.metadata();
            let ctx = crate::interceptor::build_context_from_metadata(metadata)?;
            
            // 获取请求消息
            let msg = request.into_inner().message;
            
            // 构造响应，加入上下文信息
            let response = format!("Echo: {} (from user: {:?})", msg, ctx.user_id());
            Ok(Response::new(EchoResponse { message: response }))
        }
    }

    #[tokio::test]
    async fn test_echo_with_context() -> Result<(), Box<dyn std::error::Error>> {
        // 1. 启动测试服务器
        let echo_service = EchoService;
        let addr = "127.0.0.1:50051";
        let socket_addr = SocketAddr::from_str(addr)?;

        let server_future = tonic::transport::Server::builder()
            .add_service(EchoServer::new(echo_service))
            .serve(socket_addr);
        tokio::spawn(server_future);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // 2. 创建客户端连接
        let endpoint = Channel::from_shared("http://127.0.0.1:50051")
            .map_err(|e| ServiceError::ConnectionError(e.to_string()))?
            .connect_timeout(std::time::Duration::from_secs(5))
            .tcp_keepalive(Some(std::time::Duration::from_secs(30)))
            .http2_keep_alive_interval(std::time::Duration::from_secs(30));
            
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| ServiceError::ConnectionError(e.to_string()))?;

        let client = EchoClient::new(channel);

        // 3. 准备上下文和请求参数
        let ctx = AppContextBuilder::new()
            .remote_addr("127.0.0.1:12345".to_string())
            .user_id("test-user-001".to_string())
            .platform(1)  // 假设 1 代表 Web 平台
            .client_id("test-client-001".to_string())
            .with_language(Some("zh-CN".to_string()))
            .with_conn_id("test-conn-001".to_string())
            .with_client_msg_id("test-msg-001".to_string())
            .values(Arc::new(Mutex::new({
                let mut values = HashMap::new();
                values.insert("request_id".to_string(), "test-123".to_string());
                values.insert("trace_id".to_string(), "trace-001".to_string());
                values.insert("session_id".to_string(), "session-001".to_string());
                values
            })))
            .build()
            .expect("Failed to build AppContext");

        let params = EchoRequest {
            message: "Hello".to_string(),
        };

        // 4. 调用 RPC
        let response = call_rpc(
            ctx,
            client,
            params,
            |mut client, request| Box::pin(async move {
                client.echo(request).await
            })
        ).await?;

        // 5. 验证响应
        let response_msg = response.into_inner().message;
        assert!(response_msg.contains("Hello"));
        
        Ok(())
    }
}