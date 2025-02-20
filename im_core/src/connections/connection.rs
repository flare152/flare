use crate::error;
use async_trait::async_trait;
use protobuf_codegen::{Message, Platform};

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Error,
    Authenticating,
    Authenticated,
}

/// 链接标准接口
#[async_trait]
pub trait Connection: Send + Sync {
    /// 获取连接ID
    fn id(&self) -> &str;

    /// 获取客户端ID
    fn client_id(&self) -> &str;

    /// 设置客户端ID
    fn set_client_id(&self, client_id: String);
    /// 获取用户ID
    fn user_id(&self) -> &str;

    /// 获取远程地址
    fn remote_addr(&self) -> &str;

    /// 获取平台信息
    fn platform(&self) -> Platform;

    /// 获取语言设置
    fn language(&self) -> Option<String>;

    /// 设置语言
    fn set_language(&self, language: String);

    /// 设置用户信息
    fn set_user_info(&self, user_id: String, platform: Platform, language: String);

    /// 获取连接状态
    fn state(&self) -> ConnectionState;
    /// 检查是否已认证
    fn is_authenticated(&self) -> bool;

    /// 设置认证状态
    fn set_authenticated(&self, value: bool);

    /// 发送消息
    async fn send(&self, msg: Message) -> error::Result<()>;
    /// 接收消息
    async fn receive(&self) -> error::Result<Message>;
    /// 关闭连接
    async fn close(&self) -> error::Result<()>;
}
