use async_broadcast::Receiver;
use async_trait::async_trait;
use std::sync::Arc;
use volo::discovery::{Change, Discover, Instance};
use super::{EtcdConfig, EtcdService};
use std::borrow::Cow;
use volo::context::Endpoint;
use dashmap::DashMap;
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::time::interval;
use std::time::Duration;
use std::net::SocketAddr;
use std::future::Future;
use etcd_client::Client;

#[derive(Clone)]
pub struct EtcdDiscover {
    client: Client,
    config: EtcdConfig,
    services: Arc<DashMap<String, Vec<Arc<Instance>>>>,
    broadcaster: async_broadcast::Sender<Change<String>>,
}

impl EtcdDiscover {
    pub async fn new(config: EtcdConfig) -> Result<Self, etcd_client::Error> {
        let client = config.create_client().await?;
        let (tx, _) = async_broadcast::broadcast(100);
        
        let discover = Self {
            client,
            config,
            services: Arc::new(DashMap::new()),
            broadcaster: tx,
        };

        Ok(discover)
    }

    pub async fn start_watch(&self) {
        let mut client = self.client.clone();
        let config = self.config.clone();
        let services = self.services.clone();
        let broadcaster = self.broadcaster.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(2));
            
            loop {
                interval.tick().await;
                let mut service_map: HashMap<String, Vec<Arc<Instance>>> = HashMap::new();
                
                match client.get(config.prefix.clone(), Some(etcd_client::GetOptions::new().with_prefix())).await {
                    Ok(resp) => {
                        for kv in resp.kvs() {
                            if let Ok(service) = serde_json::from_slice::<EtcdService>(kv.value()) {
                                let instances = service_map
                                    .entry(service.name.clone())
                                    .or_insert_with(Vec::new);

                                let addr = format!("{}:{}", service.address, service.port);
                                instances.push(Arc::new(Instance {
                                    address: addr.parse::<SocketAddr>().unwrap().into(),
                                    weight: 1,
                                    tags: service.tags.into_iter()
                                        .map(|tag| (Cow::Owned(tag.clone()), Cow::Owned(tag)))
                                        .collect(),
                                }));
                            }
                        }

                        // 更新缓存并发送变更通知
                        let mut old_services: Vec<String> = services.iter().map(|e| e.key().clone()).collect();
                        
                        for (service_name, instances) in service_map {
                            old_services.retain(|s| s != &service_name);
                            let old_instances = services.get(&service_name)
                                .map(|v| v.to_vec())
                                .unwrap_or_default();
                                
                            services.insert(service_name.clone(), instances.clone());

                            let changes = Change {
                                key: service_name,
                                all: instances.clone(),
                                added: instances,
                                updated: vec![],
                                removed: old_instances,
                            };

                            if let Err(e) = broadcaster.broadcast(changes).await {
                                log::error!("Failed to broadcast service changes: {}", e);
                            }
                        }

                        // 清理已不存在的服务
                        for service_name in old_services {
                            if let Some((_, old_instances)) = services.remove(&service_name) {
                                let changes = Change {
                                    key: service_name,
                                    all: vec![],
                                    added: vec![],
                                    updated: vec![],
                                    removed: old_instances,
                                };
                                if let Err(e) = broadcaster.broadcast(changes).await {
                                    log::error!("Failed to broadcast service removal: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get services: {}", e);
                        // 清空所有服务缓存
                        for item in services.iter() {
                            let service_name = item.key().clone();
                            if let Some((_, old_instances)) = services.remove(&service_name) {
                                let changes = Change {
                                    key: service_name,
                                    all: vec![],
                                    added: vec![],
                                    updated: vec![],
                                    removed: old_instances,
                                };
                                if let Err(e) = broadcaster.broadcast(changes).await {
                                    log::error!("Failed to broadcast service removal: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

#[async_trait]
impl Discover for EtcdDiscover {
    type Key = String;
    type Error = Infallible;

    fn discover<'s>(
        &'s self,
        endpoint: &'s Endpoint,
    ) -> impl Future<Output = Result<Vec<Arc<Instance>>, Self::Error>> + Send {
        async move {
            Ok(self.services
                .get(endpoint.service_name().as_str())
                .map(|v| v.to_vec())
                .unwrap_or_default())
        }
    }

    fn key(&self, endpoint: &Endpoint) -> Self::Key {
        endpoint.service_name().to_string()
    }

    fn watch(&self, _keys: Option<&[Self::Key]>) -> Option<Receiver<Change<Self::Key>>> {
        Some(self.broadcaster.new_receiver())
    }
} 