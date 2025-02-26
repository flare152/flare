use super::{EtcdConfig, EtcdService};
use crate::discover::registry::{Registration, Registry};
use async_trait::async_trait;
use etcd_client::Client;
use serde_json;
use std::time::Duration;
use tokio::time::interval;

pub struct EtcdRegistry {
    client: Client,
    config: EtcdConfig,
    ttl: Duration,
    lease_id: tokio::sync::Mutex<Option<i64>>,
}

impl EtcdRegistry {
    pub async fn new(config: EtcdConfig, ttl: Duration) -> Result<Self, etcd_client::Error> {
        let client = config.create_client().await?;
        
        Ok(Self {
            client,
            config,
            ttl,
            lease_id: tokio::sync::Mutex::new(None),
        })
    }

    async fn put_service(&self, key: &str, value: &str) -> Result<(), etcd_client::Error> {
        let mut client = self.client.clone();
        let lease =client.lease_grant(self.ttl.as_secs() as i64, None).await?;
        let lease_id = lease.id();
        let options = etcd_client::PutOptions::new().with_lease(lease_id);
        client.put(key, value, Some(options)).await?;
        
        *self.lease_id.lock().await = Some(lease_id);
        Ok(())
    }

    pub async fn start_heartbeat(&self, _service_id: String) {
        let ttl = self.ttl;
        let lease_id = *self.lease_id.lock().await;
        let client = self.client.clone();

        if let Some(lease_id) = lease_id {
            tokio::spawn(async move {
                let mut interval = interval(ttl / 2);
                let mut client = client.clone();
                loop {
                    interval.tick().await;
                    if let Err(e) = client.lease_keep_alive(lease_id).await {
                        log::error!("Failed to refresh etcd lease: {}", e);
                    }
                }
            });
        }
    }

    async fn refresh_service(&self, service_id: &str) -> Result<(), etcd_client::Error> {
        let key = format!("{}{}", self.config.prefix, service_id);
        let resp = self.client.clone().get(key.clone(), None).await?;
        if let Some(kv) = resp.kvs().first() {
            self.put_service(&key, std::str::from_utf8(kv.value()).unwrap()).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Registry for EtcdRegistry {
    type Error = std::io::Error;

    async fn register(&self, reg: Registration) -> Result<(), Self::Error> {
        let service = EtcdService {
            id: reg.id.clone(),
            name: reg.name,
            address: reg.address,
            port: reg.port,
            tags: reg.tags,
            meta: reg.meta,
        };

        let key = format!("{}{}", self.config.prefix, service.id);
        let value = serde_json::to_string(&service).unwrap();
        
        self.put_service(&key, &value)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.start_heartbeat(reg.id).await;
        
        Ok(())
    }

    async fn deregister(&self, service_id: &str) -> Result<(), Self::Error> {
        let key = format!("{}{}", self.config.prefix, service_id);
        self.client.clone()
            .delete(key, None)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    async fn heartbeat(&self, service_id: &str) -> Result<(), Self::Error> {
        self.refresh_service(service_id).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
} 