use crate::discover::registry::{Registration, Registry};
use async_trait::async_trait;
use reqwest;
use std::time::Duration;
use super::{ConsulConfig, ConsulService};
use tokio::time::interval;

pub struct ConsulRegistry {
    client: reqwest::Client,
    config: ConsulConfig,
    ttl: Duration,
}

impl ConsulRegistry {
    pub async fn new(config: ConsulConfig, ttl: Duration) -> Result<Self, reqwest::Error> {
        // 检查 Consul 是否可用
        let client = reqwest::Client::new();
        let resp = client.get(&format!("{}/v1/status/leader", config.url()))
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(resp.error_for_status().unwrap_err());
        }

        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()?;
            
        Ok(Self {
            client,
            config,
            ttl,
        })
    }

    pub async fn start_heartbeat(&self, service_id: String) {
        let client = self.client.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            let url = format!("{}/v1/agent/check/pass/service:{}", config.url(), service_id);
            
            loop {
                interval.tick().await;
                if let Err(e) = client.put(&url).send().await {
                    log::error!("Failed to send heartbeat: {}", e);
                }
            }
        });
    }

    async fn check_service_health(&self, service_id: &str) -> bool {
        let url = format!("{}/v1/agent/health/service/id/{}", self.config.url(), service_id);
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl Registry for ConsulRegistry {
    type Error = reqwest::Error;

    async fn register(&self, reg: Registration) -> Result<(), Self::Error> {
        // 先检查 Consul 是否可用
        let resp = self.client.get(&format!("{}/v1/status/leader", self.config.url()))
            .send()
            .await?;
            
        if !resp.status().is_success() {
            return Err(resp.error_for_status().unwrap_err());
        }

        let service = ConsulService {
            ID: reg.id.clone(),
            Service: reg.name,
            Tags: reg.tags,
            Address: reg.address,
            Port: reg.port,
            Meta: reg.meta,
        };

        let url = format!("{}/v1/agent/service/register", self.config.url());
        self.client.put(&url)
            .json(&service)
            .send()
            .await?;
            
        // 启动心跳
        self.start_heartbeat(reg.id).await;
        
        Ok(())
    }

    async fn deregister(&self, service_id: &str) -> Result<(), Self::Error> {
        let url = format!("{}/v1/agent/service/deregister/{}", self.config.url(), service_id);
        self.client.put(&url).send().await?;
        Ok(())
    }

    async fn heartbeat(&self, service_id: &str) -> Result<(), Self::Error> {
        let url = format!("{}/v1/agent/check/pass/service:{}", self.config.url(), service_id);
        self.client.put(&url).send().await?;
        Ok(())
    }
} 