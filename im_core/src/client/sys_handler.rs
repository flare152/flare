use crate::client::client::ClientState;
use crate::common::error::Result;
use async_trait::async_trait;
use log::debug;
use protobuf_codegen::Command;

#[async_trait]
pub trait ClientSystemHandler: Send + Sync + 'static {
    /// 退出登录
    async fn login_out(&self) -> Result<()>;

    /// 设置后台运行
    async fn set_background(&self) -> Result<()>;

    /// 设置语言
    async fn set_language(&self) -> Result<()>;

    /// 强制用户下线
    async fn kick_online(&self) -> Result<()>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;

    /// 处理连接状态变化
    async fn on_state_change(&self, state: ClientState);
    /// 获取支持的命令列表
    fn supported_commands(&self) -> Vec<Command>{
        vec![Command::LoginOut , Command::SetBackground ,
            Command::SetLanguage , Command::KickOnline ,
            Command::Close]
    }

    /// 检查是否支持某个命令
    fn supports_command(&self, command: Command) -> bool {
        self.supported_commands().contains(&command)
    }
}

/// 默认系统处理器
pub struct DefClientSystemHandler;

impl DefClientSystemHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ClientSystemHandler for DefClientSystemHandler {
    async fn login_out(&self) -> Result<()> {
        debug!("处理退出登录");
        Ok(())
    }

    async fn set_background(&self) -> Result<()> {
        debug!("处理设置后台运行");
        Ok(())
    }

    async fn set_language(&self) -> Result<()> {
        debug!("处理设置语言");
        Ok(())
    }

    async fn kick_online(&self) -> Result<()> {
        debug!("处理强制下线");
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        debug!("处理关闭连接");
        Ok(())
    }

    async fn on_state_change(&self, state: ClientState) {
        debug!("连接状态变化: {:?}", state);
    }
}