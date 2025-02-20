use crate::client::client::ClientState;
use crate::client::message_handler::MessageHandler;
use crate::client::sys_handler::{ClientSystemHandler, DefClientSystemHandler};
use crate::common::error::Result;
use log::{debug, error};
use protobuf_codegen::{Command, Response};

/// 客户端消息处理器
#[derive(Default)]
pub struct ClientMessageHandler<S , M >
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    sys_handler: S,
    msg_handler: M,
}

impl<S, M> ClientMessageHandler<S, M>
where
    S: ClientSystemHandler + Send + Sync + 'static,
    M: MessageHandler + Send + Sync + 'static,
{
    pub fn new(sys_handler: S, msg_handler: M) -> Self {
        Self {
            sys_handler,
            msg_handler,
        }
    }

    /// 创建带自定义系统处理器的实例
    pub fn with_system_handler(sys_handler: S) -> Self 
    where M: Default {
        Self::new(sys_handler, M::default())
    }

    /// 创建带自定义消息处理器的实例
    pub fn with_message_handler(msg_handler: M) -> Self 
    where S: Default {
        Self::new(S::default(), msg_handler)
    }

    /// 处理命令
    pub async fn handle_command(&self, command: Command, data: Vec<u8>) -> Result<()> {
        debug!("处理命令: {:?}", command);
        
        // 先检查是否是系统命令
        if self.sys_handler.supports_command(command) {
            match command {
                Command::LoginOut => self.sys_handler.login_out().await,
                Command::SetBackground => self.sys_handler.set_background().await,
                Command::SetLanguage => self.sys_handler.set_language().await,
                Command::KickOnline => self.sys_handler.kick_online().await,
                Command::Close => self.sys_handler.close().await,
                _ => Ok(()),
            }
        }
        // 再检查是否是消息命令
        else if self.msg_handler.supports_command(command) {
            match command {
                Command::ServerPushMsg => {
                    self.msg_handler.on_message(data).await;
                    Ok(())
                },
                Command::ServerPushCustom => {
                    self.msg_handler.on_custom_message(data).await;
                    Ok(())
                },
                Command::ServerPushNotice => {
                    self.msg_handler.on_notice_message(data).await;
                    Ok(())
                },
                Command::ServerPushData => {
                    self.msg_handler.on_data(data).await;
                    Ok(())
                },
                Command::ServerAck => {
                    self.msg_handler.on_ack_message(data).await;
                    Ok(())
                },
                _ => Ok(()),
            }
        } else {
            error!("Unsupported command: {:?}", command);
            Ok(())
        }
    }

    /// 处理响应
    pub async fn on_response(&self, msg: &Response) {
        self.msg_handler.on_response(msg).await;
    }

    /// 处理连接状态变化
    pub async fn handle_state_change(&self, state: ClientState) {
        self.sys_handler.on_state_change(state).await;
    }
}

/// 默认消息处理器
pub use crate::client::message_handler::DefMessageHandler;

impl Default for ClientMessageHandler<DefClientSystemHandler, DefMessageHandler> {
    fn default() -> Self {
        Self::new(
            DefClientSystemHandler::new(),
            DefMessageHandler::new(),
        )
    }
}
