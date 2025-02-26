use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log;


/// 服务注册接口
#[async_trait]
pub trait Registry: Send + Sync + Clone + 'static {
    type Error: std::error::Error + Send + Sync;

    /// 注册服务
    async fn register(&self, registration: Registration) -> Result<(), Self::Error>;

    /// 注销服务
    async fn deregister(&self, service_id: &str) -> Result<(), Self::Error>;

    /// 服务心跳
    async fn heartbeat(&self, service_id: &str) -> Result<(), Self::Error>;
}


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

impl Registration {
    pub fn new(
        name: String,
        id: String,
        tags: Vec<String>,
        address: String,
        port: u16,
        weight: u32,
        meta: HashMap<String, String>,
        version: String,
    ) -> Self {
        Self {
            name,
            id,
            tags,
            address,
            port,
            weight,
            meta,
            version,
        }
    }
}

/// 默认的日志注册器实现
#[derive(Debug, Clone)]
pub struct LogRegistry;

impl LogRegistry {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Registry for LogRegistry {
    type Error = std::io::Error;

    async fn register(&self, registration: Registration) -> Result<(), Self::Error> {
        log::info!("Registering service: {} (id: {})", registration.name, registration.id);
        Ok(())
    }

    async fn deregister(&self, service_id: &str) -> Result<(), Self::Error> {
        log::info!("Deregistering service: {}", service_id);
        Ok(())
    }

    async fn heartbeat(&self, service_id: &str) -> Result<(), Self::Error> {
        log::debug!("Service heartbeat: {}", service_id);
        Ok(())
    }
}