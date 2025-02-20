use crate::common::error::error::Result;
use async_trait::async_trait;
use protobuf_codegen::{Message as ProtoMessage, Platform};
use std::pin::Pin;
use std::future::Future;
use std::time::Duration;

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Error,
}

/// 链接标准接口
#[async_trait]
pub trait Connection: Send + Sync {
    /// 获取连接ID
    fn id(&self) -> &str;
    /// 远程地址
    fn remote_addr(&self) -> &str;
    /// 平台
    fn platform(&self) -> Platform;
    /// 协议名称
    fn protocol(&self) -> &str;
    /// 检查连接是否活跃
    /// 
    /// # 参数
    /// * `timeout` - 超时时间，如果最后活动时间超过此值则认为连接不活跃
    /// 
    /// # 返回
    /// * `bool` - true 表示连接活跃，false 表示连接不活跃
    async fn is_active(&self, timeout: Duration) -> bool;
    /// 发送消息
    fn send(&self, msg: ProtoMessage) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
    /// 接收消息
    fn receive(&self) -> Pin<Box<dyn Future<Output = Result<ProtoMessage>> + Send>>;
    /// 关闭连接
    fn close(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
    /// 克隆
    fn clone_box(&self) -> Box<dyn Connection>;
}
