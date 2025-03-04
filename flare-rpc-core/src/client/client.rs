use std::marker::PhantomData;
use tonic::transport::Channel;
use crate::discover::{RpcDiscovery, ServiceError};

pub trait GrpcClient {
    fn new(channel: Channel) -> Self;
}

/// RPC 客户端工厂
/// T: tonic 生成的客户端类型 (例如: echo_client::EchoClient<Channel>)
/// D: 服务发现实现
pub struct RpcClient<T, D>
where
    T: GrpcClient,
    D: RpcDiscovery,
{
    service_name: String,
    discovery: D,
    protocol: String,
    _marker: PhantomData<T>,
}

impl<T, D> RpcClient<T, D>
where
    T: GrpcClient,
    D: RpcDiscovery,
{
    pub fn new(service_name: impl Into<String>, discovery: D) -> Self {
        Self {
            service_name: service_name.into(),
            discovery,
            protocol: "http".to_string(),
            _marker: PhantomData,
        }
    }

    pub fn with_protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocol = protocol.into();
        self
    }

    /// 获取一个可用的客户端
    pub async fn client(&self) -> Result<T, ServiceError> {
        let endpoint = self.discovery.discover(&self.service_name).await?;
        
        let url = format!("{}://{}:{}", self.protocol, endpoint.address, endpoint.port);
        let channel = Channel::from_shared(url)
            .map_err(|e| ServiceError::ConnectionError(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ServiceError::ConnectionError(e.to_string()))?;

        Ok(T::new(channel))
    }

    /// 获取服务发现实例的引用
    pub fn discovery(&self) -> &D {
        &self.discovery
    }
} 