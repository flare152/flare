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
    ID: String,
    Service: String,
    Tags: Vec<String>,
    Address: String,
    Port: u16,
    Meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulServiceHealth {
    Node: ConsulNode,
    Service: ConsulService,
    Checks: Vec<ConsulCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulNode {
    Node: String,
    Address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsulCheck {
    Status: String,
} 