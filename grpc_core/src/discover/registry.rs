use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// 服务注册信息
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub name: String,
    pub id: String,
    pub tags: Vec<String>,
    pub address: String,
    pub port: u16,
    pub weight: u32,
    pub meta: HashMap<String, String>,
    pub version: String,
}

/// 服务注册接口
#[async_trait]
pub trait Registry: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync;

    /// 注册服务
    async fn register(&self, registration: Registration) -> Result<(), Self::Error>;

    /// 注销服务
    async fn deregister(&self, service_id: &str) -> Result<(), Self::Error>;

    /// 服务心跳
    async fn heartbeat(&self, service_id: &str) -> Result<(), Self::Error>;
}
