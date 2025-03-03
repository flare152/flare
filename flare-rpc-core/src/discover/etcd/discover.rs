use super::{EtcdConfig, EtcdService};
use crate::discover::discover::{Change, ServiceError};
use crate::discover::{LoadBalanceStrategy, LoadBalancer, RpcDiscovery, ServiceEndpoint};
use async_trait::async_trait;
use etcd_client::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::broadcast::{self, Sender};



#[derive(Clone)]
pub struct EtcdDiscover {
    client: Client,
    config: EtcdConfig,
    services: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
    watch_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    load_balancer: Arc<LoadBalancer>,
    broadcaster: Sender<Change>,
}

impl EtcdDiscover {
    pub async fn new(config: EtcdConfig, strategy: LoadBalanceStrategy) -> Result<Self, etcd_client::Error> {
        let client = config.create_client().await?;
        let (broadcaster, _rx) = broadcast::channel(100);
        
        Ok(Self {
            client,
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            watch_task: Arc::new(RwLock::new(None)),
            load_balancer: Arc::new(LoadBalancer::new(strategy)),
            broadcaster,
        })
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

    async fn sync_services(&self) {
        let mut client = self.client.clone();
        match client.get(
            self.config.prefix.clone(),
            Some(etcd_client::GetOptions::new().with_prefix())
        ).await {
            Ok(resp) => {
                let mut new_services = HashMap::new();
                
                // 构建新的服务列表
                for kv in resp.kvs() {
                    if let Ok(service) = serde_json::from_slice::<EtcdService>(kv.value()) {
                        let endpoints = new_services
                            .entry(service.name.clone())
                            .or_insert_with(Vec::new);
                        endpoints.push(ServiceEndpoint {
                            address: service.address,
                            port: service.port,
                            weight: service.weight,
                        });
                    }
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
            }
            Err(e) => {
                log::error!("Failed to sync services: {}", e);
                Self::clear_services(&self.services, &self.broadcaster).await;
            }
        }
    }
}

#[async_trait]
impl RpcDiscovery for EtcdDiscover {
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