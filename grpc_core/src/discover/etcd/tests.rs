#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use tokio;
    use std::time::Duration;
    use volo::context::Endpoint;
    use volo::discovery::Discover;
    use crate::discover::etcd::{EtcdConfig, EtcdDiscover, EtcdRegistry};
    use crate::discover::registry::{Registry, Registration};

    async fn setup_etcd() -> EtcdConfig {
        let config = EtcdConfig {
            addr: "http://127.0.0.1:2379".to_string(),
            timeout: Duration::from_secs(5),
            prefix: "/services/".to_string(),
            username: None,
            password: None,
        };

        // 等待 etcd 启动
        let client = reqwest::Client::new();
        let mut retries = 5;
        while retries > 0 {
            if let Ok(resp) = client.get(&format!("{}/version", config.addr)).send().await {
                if resp.status().is_success() {
                    return config;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            retries -= 1;
        }

        panic!("Etcd is not available");
    }

    #[tokio::test]
    async fn test_etcd_health_check() {
        let config = setup_etcd().await;
        let client = reqwest::Client::new();
        let resp = client.get(&format!("{}/version", config.addr))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }

    #[tokio::test]
    async fn test_etcd_registry() {
        let config = setup_etcd().await;
        let mut registry = EtcdRegistry::new(config.clone(), Duration::from_secs(30)).await.unwrap();
        
        let reg = Registration {
            id: "test-service-1".to_string(),
            name: "test-service".to_string(),
            address: "127.0.0.1".to_string(),
            port: 8080,
            tags: vec!["test".to_string()],
            meta: HashMap::new(),
            weight: 1,
            version: "1.0".into(),
        };

        registry.register(reg).await.unwrap();

        let discover = EtcdDiscover::new(config).await.unwrap();
        discover.start_watch().await;

        let endpoint = Endpoint::new("test-service".to_string().parse().unwrap());
        
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        let instances = discover.discover(&endpoint).await.unwrap();
        assert!(!instances.is_empty());

        // 测试注销
        registry.deregister("test-service-1").await.unwrap();
        
        // 等待注销生效
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // 验证服务已被注销
        let instances = discover.discover(&endpoint).await.unwrap();
        assert!(instances.is_empty());
    }
} 