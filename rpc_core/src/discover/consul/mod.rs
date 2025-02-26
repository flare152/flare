mod discover;
mod registry;
mod tests;

pub use discover::ConsulDiscover;
pub use registry::ConsulRegistry;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulConfig {
    pub addr: String,
    pub timeout: Duration,
    pub protocol: String,
    pub token: Option<String>,
}

impl Default for ConsulConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:8500".to_string(),
            timeout: Duration::from_secs(30),
            protocol: "http".to_string(),
            token: None,
        }
    }
}

impl ConsulConfig {
    pub fn url(&self) -> String {
        format!("{}://{}", self.protocol, self.addr)
    }

    pub async fn check_health(&self) -> bool {
        let client = reqwest::Client::new();
        match client.get(&format!("{}/v1/status/leader", self.url())).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulService {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Service")]
    service: String,
    #[serde(rename = "Tags")]
    tags: Vec<String>,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "Port")]
    port: u16,
    #[serde(rename = "Meta")]
    meta: HashMap<String, String>,
    #[serde(rename = "EnableTagOverride", skip_serializing)]
    enable_tag_override: Option<bool>,
    #[serde(rename = "CreateIndex", skip_serializing)]
    create_index: Option<u64>,
    #[serde(rename = "ModifyIndex", skip_serializing)]
    modify_index: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulServiceHealth {
    #[serde(rename = "Node")]
    node: ConsulNode,
    #[serde(rename = "Service")]
    service: ConsulService,
    #[serde(rename = "Checks")]
    checks: Vec<ConsulCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulNode {
    #[serde(rename = "Node")]
    node: String,
    #[serde(rename = "Address")]
    address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulCheck {
    #[serde(rename = "TTL")]
    ttl: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "DeregisterCriticalServiceAfter")]
    deregister_critical_service_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterService {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Tags")]
    tags: Vec<String>,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "Port")]
    port: u16,
    #[serde(rename = "Meta")]
    meta: HashMap<String, String>,
    #[serde(rename = "Check")]
    check: ConsulCheck,
} 