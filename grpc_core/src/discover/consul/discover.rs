use async_broadcast::Receiver;
use async_trait::async_trait;
use reqwest;
use std::sync::Arc;
use volo::discovery::{Change, Discover, Instance};
use super::{ConsulConfig, ConsulService, ConsulServiceHealth};
use std::borrow::Cow;
use volo::context::Endpoint;
use dashmap::DashMap;
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::time::interval;
use std::time::Duration;
use std::net::SocketAddr;
use std::future::Future;

#[derive(Clone)]
pub struct ConsulDiscover {
    client: reqwest::Client,
    config: ConsulConfig,
    services: Arc<DashMap<String, Vec<Arc<Instance>>>>,
    broadcaster: async_broadcast::Sender<Change<String>>,
}

impl ConsulDiscover {
    pub fn new(config: ConsulConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap();
            
        let (tx, _) = async_broadcast::broadcast(100);
        
        Self {
            client,
            config,
            services: Arc::new(DashMap::new()),
            broadcaster: tx,
        }
    }

    pub async fn start_watch(&self) {
        let client = self.client.clone();
        let config = self.config.clone();
        let services = self.services.clone();
        let broadcaster = self.broadcaster.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                let mut service_map: HashMap<String, Vec<Arc<Instance>>> = HashMap::new();
                
                if let Ok(response) = client.get(&format!("{}/v1/agent/services", config.url())).send().await {
                    if let Ok(services_map) = response.json::<HashMap<String, ConsulService>>().await {
                        for (_, service) in services_map {
                            let instances = service_map
                                .entry(service.service.clone())
                                .or_insert_with(|| Vec::new());

                            let addr = format!("{}:{}", service.address, service.port);
                                
                            instances.push(Arc::new(Instance {
                                address: addr.parse::<SocketAddr>().unwrap().into(),
                                weight: 1,
                                tags: service.tags.into_iter()
                                    .map(|tag| (Cow::Owned(tag.clone()), Cow::Owned(tag)))
                                    .collect(),
                            }));
                        }
                    } else {
                        log::error!("Failed to parse services response");
                    }
                }

                // 更新缓存并发送变更通知
                for (service_name, instances) in service_map {
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
            }
        });
    }
}

#[async_trait]
impl Discover for ConsulDiscover {
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