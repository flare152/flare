use std::time::Duration;
use protobuf_codegen::Platform;

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);
const RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
const MAX_RECONNECT_ATTEMPTS: u32 = 5;

/// 客户端配置
#[derive(Clone)]
pub struct ClientConfig {
    pub ping_interval: Duration,
    pub pong_timeout: Duration,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub auth_token: String,
    pub platform: Platform,
    pub client_id: String,
    pub user_id: String,
    pub language: Option<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            ping_interval: PING_INTERVAL,
            pong_timeout: PONG_TIMEOUT,
            reconnect_interval: RECONNECT_INTERVAL,
            max_reconnect_attempts: MAX_RECONNECT_ATTEMPTS,
            auth_token: String::new(),
            platform: Platform::Unknown,
            client_id: uuid::Uuid::new_v4().to_string(),
            user_id: String::new(),
            language: None,
        }
    }
}

/// 客户端配置构建器
pub struct ClientConfigBuilder {
    config: ClientConfig,
}

impl ClientConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    /// 设置心跳间隔
    pub fn ping_interval(mut self, interval: Duration) -> Self {
        self.config.ping_interval = interval;
        self
    }

    /// 设置 PONG 超时时间
    pub fn pong_timeout(mut self, timeout: Duration) -> Self {
        self.config.pong_timeout = timeout;
        self
    }

    /// 设置重连间隔
    pub fn reconnect_interval(mut self, interval: Duration) -> Self {
        self.config.reconnect_interval = interval;
        self
    }

    /// 设置最大重连次数
    pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.config.max_reconnect_attempts = attempts;
        self
    }

    /// 设置认证令牌
    pub fn auth_token(mut self, token: impl Into<String>) -> Self {
        self.config.auth_token = token.into();
        self
    }

    /// 设置平台
    pub fn platform(mut self, platform: Platform) -> Self {
        self.config.platform = platform;
        self
    }

    /// 设置客户端ID
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.config.client_id = client_id.into();
        self
    }

    /// 设置用户ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.config.user_id = user_id.into();
        self
    }

    /// 设置语言
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.config.language = Some(language.into());
        self
    }

    /// 构建配置
    pub fn build(self) -> ClientConfig {
        self.config
    }
}

impl Default for ClientConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfigBuilder::new()
            .ping_interval(Duration::from_secs(60))
            .pong_timeout(Duration::from_secs(20))
            .reconnect_interval(Duration::from_secs(10))
            .max_reconnect_attempts(3)
            .auth_token("test_token")
            .platform(Platform::Web)
            .client_id("test_client")
            .user_id("test_user")
            .language("zh-CN")
            .build();

        assert_eq!(config.ping_interval, Duration::from_secs(60));
        assert_eq!(config.pong_timeout, Duration::from_secs(20));
        assert_eq!(config.reconnect_interval, Duration::from_secs(10));
        assert_eq!(config.max_reconnect_attempts, 3);
        assert_eq!(config.auth_token, "test_token");
        assert_eq!(config.platform, Platform::Web);
        assert_eq!(config.client_id, "test_client");
        assert_eq!(config.user_id, "test_user");
        assert_eq!(config.language, Some("zh-CN".to_string()));
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();

        assert_eq!(config.ping_interval, PING_INTERVAL);
        assert_eq!(config.pong_timeout, PONG_TIMEOUT);
        assert_eq!(config.reconnect_interval, RECONNECT_INTERVAL);
        assert_eq!(config.max_reconnect_attempts, MAX_RECONNECT_ATTEMPTS);
        assert!(config.auth_token.is_empty());
        assert_eq!(config.platform, Platform::Unknown);
        assert!(!config.client_id.is_empty());
        assert!(config.user_id.is_empty());
        assert_eq!(config.language, None);
    }
} 