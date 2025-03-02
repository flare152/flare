use crate::app::AppConfig;
use crate::{Registration, Registry};
use anyhow;
use log::{error, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::oneshot;
use uuid;
use crate::discover::LogRegistry;
use tonic::transport::Server;
use std::future::Future;

pub type DefaultApp = App<LogRegistry>;

/// RPC 应用程序
pub struct App<R>
where
    R: Registry,
{
    /// 应用配置
    pub config: AppConfig,
    /// 服务注册器
    register: Option<R>,
}

impl<R> App<R>
where
    R: Registry,
{
    /// 创建新的应用实例
    ///
    /// # Arguments
    /// * `id` - 应用ID
    /// * `name` - 应用名称
    /// * `version` - 应用版本
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        Self {
            config: AppConfig {
                id: id.to_string(),
                name: name.to_string(),
                version: version.to_string(),
                ..Default::default()
            },
            register: None,
        }
    }

    /// 创建简单应用实例（使用随机ID）
    pub fn new_simple(name: &str) -> Self {
        Self {
            config: AppConfig {
                name: name.to_string(),
                ..Default::default()
            },
            register: None,
        }
    }

    /// 创建不需要注册的应用实例
    pub fn new_not_register(id: &str, name: &str, version: &str) -> Self {
        let mut app = Self::new(id, name, version);
        app.config.weight = 10;
        app
    }

    /// 创建简单的不需要注册的应用实例
    pub fn new_simple_not_register(name: &str) -> Self {
        let mut app = Self::new_simple(name);
        app.config.weight = 10;
        app
    }

    /// 设置服务注册器
    pub fn register(mut self, register: R) -> Self {
        self.register = Some(register);
        self
    }
    

    /// 添加应用标签
    pub fn add_tag(&mut self, tag: &str) -> &mut Self {
        self.config.tags.push(tag.to_string());
        self
    }

    /// 添加元数据
    pub fn add_meta(&mut self, key: &str, value: &str) -> &mut Self {
        self.config.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// 设置服务权重
    pub fn set_weight(&mut self, weight: u32) -> &mut Self {
        self.config.weight = weight;
        self
    }

    /// 运行服务器
    ///
    /// # Arguments
    /// * `ip` - 监听IP地址
    /// * `port` - 监听端口
    /// * `server_fn` - 服务器函数
    pub async fn run<F, Fut>(self, ip: &str, port: u16, server_fn: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce(Server) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        // 准备注册信息
        let registration = Registration::new(
            self.config.name.clone(),
            self.config.id.clone(),
            self.config.tags,
            ip.to_string(),
            port,
            self.config.weight,
            self.config.metadata,
            self.config.version,
        );
        
        let register = if let Some(r) = self.register {
            r
        } else {
            return Err("No registry provided".into());
        };
        
        // 处理服务注册
        register_server(register.clone(), registration).await?;

        // 启动心跳检查
        let service_id = self.config.id.clone();
        let register_clone = register.clone();
        let heartbeat_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                if let Err(e) = register_clone.heartbeat(&service_id).await {
                    log::error!("Heartbeat failed: {}", e);
                }
            }
        });

        // 启动服务器
        let addr: SocketAddr = format!("{}:{}", ip, port).parse()?;
        let server = Server::builder();
        info!("Starting server at: {}", addr);

        // 创建关闭信号通道
        let (tx, rx) = oneshot::channel();
        
        // 启动服务器
        let server_handle = tokio::spawn(server_fn(server));

        // 监听关闭信号
        tokio::spawn(async move {
            let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = term.recv() => {},
            }
            let _ = tx.send(());
        });

        // 等待关闭信号
        rx.await.unwrap();
        info!("Shutting down gracefully...");

        // 停止心跳
        heartbeat_handle.abort();

        // 注销服务
        deregister_server(register, self.config.id).await?;

        // 等待服务器关闭
        server_handle.await??;
        Ok(())
    }
}

/// 注册服务
async fn register_server<R>(registry: R, reg: Registration) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: Registry,
{
    registry.register(reg).await?;
    Ok(())
}

/// 注销服务
async fn deregister_server<R>(registry: R, id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: Registry,
{
    match registry.deregister(id.as_str()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to deregister service: {}", e);
            Err(anyhow::anyhow!("Failed to deregister service: {}", e).into())
        }
    }
}

/// App Builder
pub struct AppBuilder<R>
where
    R: Registry,
{
    id: Option<String>,
    name: String,
    version: Option<String>,
    weight: u32,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
    register: Option<R>,
}

impl<R> AppBuilder<R>
where
    R: Registry,
{
    /// 创建新的 Builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            version: None,
            weight: 1,
            tags: Vec::new(),
            metadata: HashMap::new(),
            register: None,
        }
    }

    /// 设置应用 ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设置版本
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// 设置权重
    pub fn weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    /// 添加标签
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 添加元数据
    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 设置注册器
    pub fn register(mut self, register: R) -> Self {
        self.register = Some(register);
        self
    }

    /// 构建 App 实例
    pub fn build(self) -> App<R> {
        App {
            config: AppConfig {
                id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                name: self.name,
                version: self.version.unwrap_or_else(|| "0.1.0".to_string()),
                weight: self.weight,
                tags: self.tags,
                metadata: self.metadata,
            },
            register: self.register,
        }
    }
}

impl<R> App<R>
where
    R: Registry,
{
    /// 创建新的 Builder
    pub fn builder(name: impl Into<String>) -> AppBuilder<R> {
        AppBuilder::new(name)
    }
}
