#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use tokio;
    use std::time::Duration;
    use volo::context::Endpoint;
    use volo::discovery::Discover;
    use crate::discover::consul::{ConsulConfig, ConsulRegistry};
    use crate::discover::ConsulDiscover;
    use crate::discover::registry::Registry;
    use crate::Registration;

    async fn setup_consul() -> ConsulConfig {
        let config = ConsulConfig {
            addr: "127.0.0.1:8500".to_string(),
            timeout: Duration::from_secs(5),
            protocol: "http".to_string(),
            token: None,
        };

        // 等待 Consul 启动
        let mut retries = 5;
        while !config.check_health().await && retries > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            retries -= 1;
        }

        if !config.check_health().await {
            panic!("Consul is not available");
        }

        config
    }

    #[tokio::test]
    async fn test_consul_health_check() {
        let config = setup_consul().await;
        assert!(config.check_health().await);
    }

    #[tokio::test]
    async fn test_consul_registry() {
        let config = setup_consul().await;
        let registry = ConsulRegistry::new(config.clone(), Duration::from_secs(30)).await.unwrap();
        
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

        // 测试注册
        registry.register(reg).await.unwrap();

        // 测试发现
        let discover = ConsulDiscover::new(config);
        discover.start_watch().await;

        let endpoint = Endpoint::new("test-service".to_string().parse().unwrap());
        
        // 等待服务发现更新
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        let instances = discover.discover(&endpoint).await.unwrap();
        assert!(!instances.is_empty());

        // 测试注销
        registry.deregister("test-service-1").await.unwrap();
    }
} 