mod client;
mod tests;
mod call;

use tonic::transport::Channel;
use flare::context::AppContext;
use crate::interceptor::ctxinterceprot::AppContextInterceptor;
use tonic::{Request, Status, Response};
use std::future::Future;
use std::pin::Pin;

pub use client::*;

pub async fn call_rpc<C, P, R>(
    ctx: AppContext,
    mut client: C,
    params: P,
    rpc_call: impl FnOnce(&mut C, Request<P>) -> Pin<Box<dyn Future<Output = Result<Response<R>, Status>> + Send>>,
) -> Result<Response<R>, Status> {
    // 创建拦截器并设置上下文
    let interceptor = AppContextInterceptor::new();
    interceptor.set_context(ctx);

    // 创建请求并添加拦截器
    let request = Request::new(params);
    let request = interceptor.call(request.into_request())?;

    // 执行 RPC 调用
    rpc_call(&mut client, request).await
}

// 使用示例
#[cfg(test)]
mod rpc_tests {
    use super::*;

    #[tokio::test]
    async fn test_call_rpc() -> Result<(), Box<dyn std::error::Error>> {
        // 模拟的 gRPC 客户端和请求/响应类型
        struct TestClient {}
        struct TestRequest {
            message: String,
        }
        struct TestResponse {
            message: String,
        }

        impl TestClient {
            async fn test(&mut self, request: Request<TestRequest>) -> Result<Response<TestResponse>, Status> {
                Ok(Response::new(TestResponse {
                    message: request.into_inner().message,
                }))
            }
        }

        // 创建测试上下文
        let ctx = AppContext::default();
        
        // 创建测试客户端和参数
        let client = TestClient {};
        let params = TestRequest {
            message: "test".to_string(),
        };

        // 调用 RPC
        let result = call_rpc(
            ctx,
            client,
            params,
            |client, request| Box::pin(client.test(request)),
        ).await?;

        assert_eq!(result.into_inner().message, "test");
        Ok(())
    }
}