use std::collections::HashMap;
use uuid::Uuid;

mod app;
mod tests;

pub use app::{App, AppBuilder, DefaultApp};

/// RPC 应用程序配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 应用唯一标识
    pub id: String,
    /// 应用名称
    pub name: String,
    /// 应用版本
    pub version: String,
    /// 应用元数据
    pub metadata: HashMap<String, String>,
    /// 应用标签
    pub tags: Vec<String>,
    /// 服务权重
    pub weight: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            version: "1.0.0".to_string(),
            metadata: HashMap::new(),
            tags: Vec::new(),
            weight: 1,
        }
    }
}
