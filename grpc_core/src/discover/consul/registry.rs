use crate::discover::registry::{Registration, Registry};
use async_trait::async_trait;
use reqwest;
use std::time::Duration;
use super::{ConsulConfig, ConsulService, ConsulCheck, RegisterService};
use tokio::time::interval;
use serde::Serialize;

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

    async fn request<T: Serialize>(&self, method: reqwest::Method, path: &str, body: Option<&T>) -> Result<(), reqwest::Error> {
        let url = format!("{}{}", self.config.url(), path);
        let mut req = self.client.request(method, &url);
        
        if let Some(data) = body {
            req = req.json(data);
        }
        
        let res = req.send().await?;
        if !res.status().is_success() {
            let err = res.error_for_status().unwrap_err();
            let status = err.status().unwrap_or_default();
            
            log::error!(
                "Consul API 错误: 路径={}, 状态码={}", 
                path, status
            );
            
            return Err(err);
        }
        
        Ok(())
    }
}

#[async_trait]
impl Registry for ConsulRegistry {
    type Error = reqwest::Error;

    async fn register(&self, reg: Registration) -> Result<(), Self::Error> {
        // 先检查 Consul 是否可用
        self.request::<()>(reqwest::Method::GET, "/v1/status/leader", None).await?;

        let service = RegisterService {
            id: reg.id.clone(),
            name: reg.name,
            tags: reg.tags,
            address: reg.address,
            port: reg.port,
            meta: reg.meta,
            check: ConsulCheck {
                ttl: format!("{}s", self.ttl.as_secs()),
                status: "passing".to_string(),
                deregister_critical_service_after: "24h".to_string(),
            },
        };

        self.request::<RegisterService>(
            reqwest::Method::PUT,
            "/v1/agent/service/register",
            Some(&service)
        ).await?;
        
        // 启动心跳
        self.start_heartbeat(reg.id).await;
        
        Ok(())
    }

    async fn deregister(&self, service_id: &str) -> Result<(), Self::Error> {
        self.request::<()>(
            reqwest::Method::PUT,
            &format!("/v1/agent/service/deregister/{}", service_id),
            None
        ).await
    }

    async fn heartbeat(&self, service_id: &str) -> Result<(), Self::Error> {
        self.request::<()>(
            reqwest::Method::PUT,
            &format!("/v1/agent/check/pass/service:{}", service_id),
            None
        ).await
    }
} 