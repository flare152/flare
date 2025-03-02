use async_trait::async_trait;
use rand::seq::IndexedRandom;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;


#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service not found: {0}")]
    NotFound(String),
    
    #[error("connection error: {0}")]
    ConnectionError(String),
    
    #[error("decode error: {0}")]
    DecodeError(String),
    
    #[error("resource error: {0}")]
    ResourceError(String),
}

#[derive(Clone, Copy, Debug)]
pub enum LoadBalanceStrategy {
    Random,
    RoundRobin,
    WeightedRandom,
}

#[async_trait]
pub trait RpcDiscovery: Send + Sync + Clone + 'static {
    /// 发现服务并返回连接通道
    async fn discover(&self, service_name: &str) -> Result<ServiceEndpoint, ServiceError>;

    /// 启动服务发现监听
    async fn start_watch(&self);

    /// 停止服务发现监听
    async fn stop_watch(&self);
}

#[derive(Clone)]
pub struct ServiceEndpoint {
    pub url: String,
    pub weight: u32,
}

#[derive(Clone)]
pub struct LoadBalancer {
    strategy: LoadBalanceStrategy,
    round_robin_index: Arc<Mutex<HashMap<String, usize>>>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            strategy,
            round_robin_index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn select_endpoint(&self, service_name: &str, endpoints: &[ServiceEndpoint]) -> Option<ServiceEndpoint> {
        if endpoints.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::Random => {
                let mut rng = rand::rng();
                endpoints.choose(&mut rng).map(|ep| ep.clone())
            },
            LoadBalanceStrategy::WeightedRandom => {
                let total_weight: u32 = endpoints.iter().map(|ep| ep.weight).sum();
                if total_weight == 0 {
                    return endpoints.choose(&mut rand::rng()).map(|ep| ep.clone());
                }

                let mut rng = rand::rng();
                let chosen_weight = rng.random_range(0..total_weight);
                let mut accumulated_weight = 0;
                
                // 累加权重,权重大的服务占据更大的随机空间
                for endpoint in endpoints {
                    accumulated_weight += endpoint.weight;
                    if chosen_weight < accumulated_weight {
                        return Some(endpoint.clone());
                    }
                }
                
                // 保底返回第一个服务
                Some(endpoints[0].clone())
            },
            LoadBalanceStrategy::RoundRobin => {
                let mut indices = self.round_robin_index.lock().await;
                let index = indices.entry(service_name.to_string())
                    .and_modify(|i| *i = (*i + 1) % endpoints.len())
                    .or_insert(0);
                Some(endpoints[*index].clone())
            }
        }
    }
}

#[derive(Clone)]
pub struct Change {
    pub service_name: String,
    pub all: Vec<ServiceEndpoint>,
    pub added: Vec<ServiceEndpoint>,
    pub updated: Vec<ServiceEndpoint>,
    pub removed: Vec<ServiceEndpoint>,
}