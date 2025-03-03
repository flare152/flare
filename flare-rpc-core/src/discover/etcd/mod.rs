mod discover;
mod registry;
mod tests;

pub use discover::EtcdDiscover;
pub use registry::EtcdRegistry;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use etcd_client::{Client, ConnectOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdConfig {
    pub addr: String,
    pub timeout: Duration,
    pub prefix: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            addr: "http://127.0.0.1:2379".to_string(),
            timeout: Duration::from_secs(30),
            prefix: "/services/".to_string(),
            username: None,
            password: None,
        }
    }
}

impl EtcdConfig {
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    pub async fn create_client(&self) -> Result<Client, etcd_client::Error> {
        let mut options = ConnectOptions::new();
        options = options.with_timeout(self.timeout);
        
        let options = if let (Some(username), Some(password)) = (&self.username, &self.password) {
            options.with_user(username, password)
        } else {
            options
        };
        
        Client::connect([self.addr.clone()], Some(options)).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EtcdService {
    id: String,
    name: String,
    tags: Vec<String>,
    address: String,
    port: u16,
    weight: u32,
    meta: HashMap<String, String>,
    version: String,
}

use std::collections::HashMap; 