#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{LoadBalanceStrategy, RpcDiscovery};
    use crate::discover::discover::ServiceError;
    use crate::discover::registry::{Registry, Registration};
    use crate::discover::etcd::{EtcdConfig, EtcdRegistry, EtcdDiscover};
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio;
    use tonic::transport::Channel;

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
    async fn test_etcd_service_lifecycle() {
        let config = setup_etcd().await;
        
        // 创建注册器
        let registry = EtcdRegistry::new(config.clone(), Duration::from_secs(30)).await.expect("Failed to create registry");
        
        // 创建服务发现器
        let discover = EtcdDiscover::new(config.clone(), LoadBalanceStrategy::RoundRobin).await.expect("Failed to create discover");
        discover.start_watch().await;

        // 注册服务
        let registration = Registration::new(
            "test-service".to_string(),
            "test-instance-1".to_string(),
            vec!["test".to_string()],
            "127.0.0.1".to_string(),
            8080,
            3,
            HashMap::new(),
            "1.0.0".to_string(),
        );

        registry.register(registration).await.expect("Failed to register service");

        // 等待服务发现更新
        tokio::time::sleep(Duration::from_secs(20)).await;

        // 测试服务发现
        let endpoint = discover.discover("test-service").await.expect("Failed to discover service");
        assert_eq!(endpoint.url, "http://127.0.0.1:8080");
        assert_eq!(endpoint.weight, 3);

        // 测试注销服务
        registry.deregister("test-instance-1").await.expect("Failed to deregister service");

        // 等待服务发现更新
        tokio::time::sleep(Duration::from_secs(20)).await;

        // 验证服务已被注销
        match discover.discover("test-service").await {
            Ok(_) => panic!("Service should not be found after deregistration"),
            Err(e) => match e {
                ServiceError::NotFound(_) => (),
                _ => panic!("Expected NotFound error, got: {}", e),
            },
        }
    }

    #[tokio::test]
    async fn test_load_balance_strategies() {
        let config = setup_etcd().await;
        
        // 创建注册器
        let registry = EtcdRegistry::new(config.clone(), Duration::from_secs(30)).await.expect("Failed to create registry");
        
        // 注册多个服务实例
        for i in 1..=3 {
            let registration = Registration::new(
                "test-lb-service".to_string(),
                format!("test-instance-{}", i),
                vec!["test".to_string()],
                "127.0.0.1".to_string(),
                8080 + i as u16,
                3,
                HashMap::new(),
                "1.0.0".to_string(),
            );
            registry.register(registration).await.expect("Failed to register service");
        }

        // 测试轮询策略
        let discover_rr = EtcdDiscover::new(config.clone(), LoadBalanceStrategy::RoundRobin).await.expect("Failed to create discover");
        discover_rr.start_watch().await;
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let mut ports = Vec::new();
        for _ in 0..3 {
            let endpoint = discover_rr.discover("test-lb-service").await.expect("Failed to discover service");
            let port = endpoint.url.split(':').last().unwrap().parse::<u16>().unwrap();
            ports.push(port);
        }
        
        // 验证轮询是否访问了不同端口
        assert_eq!(ports.len(), 3);
        assert!(ports.windows(2).all(|w| w[0] != w[1]), "Expected different ports in sequence");

        // 测试随机策略
        let discover_random = EtcdDiscover::new(config.clone(), LoadBalanceStrategy::Random).await.expect("Failed to create discover");
        discover_random.start_watch().await;
        
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 验证随机策略能够发现服务
        let endpoint = discover_random.discover("test-lb-service").await.expect("Failed to discover service with random strategy");
        assert!(endpoint.url.starts_with("http://127.0.0.1:"));
        assert_eq!(endpoint.weight, 1);
        
        // 清理测试服务
        for i in 1..=3 {
            registry.deregister(&format!("test-instance-{}", i)).await.expect("Failed to deregister service");
        }
    }
} 