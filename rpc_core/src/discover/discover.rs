use async_trait::async_trait;

#[async_trait]
pub trait RpcDiscovery: Send + Sync + Clone + 'static {
    /// 启动监听
    async fn start_watch(&self) ;
}

