use super::{ConsulConfig, ConsulService};
use crate::discover::discover::{Change, ServiceError};
use crate::discover::{LoadBalanceStrategy, LoadBalancer, RpcDiscovery, ServiceEndpoint};
use async_trait::async_trait;
use reqwest;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::broadcast::{self, Sender};


#[derive(Clone)]
pub struct ConsulDiscover {
    client: reqwest::Client,
    config: ConsulConfig,
    services: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
    watch_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    load_balancer: Arc<LoadBalancer>,
    broadcaster: Sender<Change>,
}

impl ConsulDiscover {
    pub fn new(config: ConsulConfig, strategy: LoadBalanceStrategy) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap();
            
        let (broadcaster, _rx) = broadcast::channel(100);
            
        Self {
            client,
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            watch_task: Arc::new(RwLock::new(None)),
            load_balancer: Arc::new(LoadBalancer::new(strategy)),
            broadcaster,
        }
    }

    async fn clear_services(services: &Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>, broadcaster: &Sender<Change>) {
        let mut services_lock = services.write().await;
        for (service_name, old_endpoints) in services_lock.drain() {
            let change = Change {
                service_name,
                all: vec![],
                added: vec![],
                updated: vec![],
                removed: old_endpoints,
            };
            if let Err(e) = broadcaster.send(change) {
                log::error!("Failed to broadcast service removal: {}", e);
            }
        }
    }
    // 同步服务列表
    async fn sync_services(&self) {
        // 获取通过健康检查的服务
        let health_url = format!("{}/v1/health/state/passing", self.config.url());
        let mut health_request = self.client.get(&health_url);
        if let Some(token) = &self.config.token {
            health_request = health_request.header("X-Consul-Token", token);
        }
        
        let healthy_services = match health_request.send().await {
            Ok(health_response) => {
                match health_response.json::<Vec<serde_json::Value>>().await {
                    Ok(checks) => {
                        // 获取所有通过健康检查的服务 ID
                        checks.iter()
                            .filter_map(|check| {
                                check.get("ServiceID")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect::<std::collections::HashSet<String>>()
                    }
                    Err(e) => {
                        log::error!("Failed to parse health checks response: {}", e);
                        std::collections::HashSet::new()
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to fetch health checks: {}", e);
                std::collections::HashSet::new()
            }
        };

        // 获取服务详情
        let url = format!("{}/v1/agent/services", self.config.url());
        let mut request = self.client.get(&url);
        if let Some(token) = &self.config.token {
            request = request.header("X-Consul-Token", token);
        }
        
        match request.send().await {
            Ok(response) => {
                if let Ok(services_map) = response.json::<HashMap<String, ConsulService>>().await {
                    let mut new_services = HashMap::new();
                    
                    // 构建新的服务列表，只包含健康的服务
                    for (id, service) in services_map {
                        if !healthy_services.contains(&id) {
                            continue;
                        }
                        
                        let endpoints = new_services
                            .entry(service.service.clone())
                            .or_insert_with(Vec::new);
                            
                        let weight = service.meta.get("weight")
                            .and_then(|w| w.parse::<u32>().ok())
                            .unwrap_or(1);
                            
                        endpoints.push(ServiceEndpoint {
                            address: service.address.clone(),
                            port: service.port,
                            weight,
                        });
                    }
                    
                    // 获取旧服务列表
                    let mut services_lock = self.services.write().await;
                    let mut old_services: Vec<String> = services_lock.keys().cloned().collect();
                    
                    // 处理服务变更
                    for (service_name, new_endpoints) in new_services {
                        old_services.retain(|s| s != &service_name);
                        
                        let old_endpoints = services_lock.get(&service_name)
                            .cloned()
                            .unwrap_or_default();
                            
                        // 计算新增和移除的端点
                        let added: Vec<_> = new_endpoints.iter()
                            .filter(|ep| !old_endpoints.iter().any(|old| 
                                old.address == ep.address && old.port == ep.port
                            ))
                            .cloned()
                            .collect();
                            
                        let removed: Vec<_> = old_endpoints.iter()
                            .filter(|ep| !new_endpoints.iter().any(|new| 
                                new.address == ep.address && new.port == ep.port
                            ))
                            .cloned()
                            .collect();
                            
                        // 只有在有变更时才发送通知
                        if !added.is_empty() || !removed.is_empty() {
                            // 更新服务列表
                            services_lock.insert(service_name.clone(), new_endpoints.clone());
                            
                            // 发送变更通知
                            let change = Change {
                                service_name,
                                all: new_endpoints,
                                added,
                                updated: vec![],
                                removed,
                            };
                            
                            // 检查是否有接收者
                            if self.broadcaster.receiver_count() > 0 {
                                if let Err(e) = self.broadcaster.send(change) {
                                    log::debug!("Failed to broadcast service changes: {}", e);
                                }
                            }
                        }
                    }
                    
                    // 处理已删除的服务
                    for service_name in old_services {
                        if let Some(old_endpoints) = services_lock.remove(&service_name) {
                            let change = Change {
                                service_name,
                                all: vec![],
                                added: vec![],
                                updated: vec![],
                                removed: old_endpoints,
                            };
                            
                            // 检查是否有接收者
                            if self.broadcaster.receiver_count() > 0 {
                                if let Err(e) = self.broadcaster.send(change) {
                                    log::debug!("Failed to broadcast service removal: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    log::error!("Failed to parse services response");
                    Self::clear_services(&self.services, &self.broadcaster).await;
                }
            }
            Err(e) => {
                log::error!("Failed to sync services: {}", e);
                Self::clear_services(&self.services, &self.broadcaster).await;
            }
        }
    }
}

#[async_trait]
impl RpcDiscovery for ConsulDiscover {
    async fn discover(&self, service_name: &str) -> Result<ServiceEndpoint, ServiceError> {
        let services = self.services.read().await;
        let endpoints = services.get(service_name)
            .ok_or_else(|| ServiceError::NotFound(service_name.to_string()))?;

        if endpoints.is_empty() {
            return Err(ServiceError::NotFound(format!("No endpoints found for service {}", service_name)));
        }
        self.load_balancer.select_endpoint(service_name, endpoints).await
            .ok_or_else(|| ServiceError::ResourceError("Failed to choose endpoint".to_string()))
    }

    async fn start_watch(&self) {
        // 首次同步服务列表
        self.sync_services().await;

        let this = self.clone();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            
            loop {
                interval.tick().await;
                this.sync_services().await;
            }
        });

        *self.watch_task.write().await = Some(task);
    }

    async fn stop_watch(&self) {
        if let Some(task) = self.watch_task.write().await.take() {
            task.abort();
        }
    }
} 