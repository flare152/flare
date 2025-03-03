use flare_core::context::AppContext;
use flare_core::error::{FlareErr, Result};
use async_trait::async_trait;
use log::debug;
use flare_core::flare_net::net::{Command, ResCode, Response};

use crate::server::auth_handler::{AuthCommandHandler, AuthHandler};
use crate::server::server::ConnectionInfo;
use crate::server::server_handler::{ ServerCommandHandler, ServerHandler};
use crate::server::sys_handler::{SystemCommandHandler, SystemHandler};

/// 命令处理器 trait
#[async_trait]
pub trait CommandHandler: Send + Sync {
    /// 处理命令
    async fn handle_command(&self, ctx:  &AppContext) -> Result<Response>;
    
    /// 获取支持的命令列表
    fn supported_commands(&self) -> Vec<Command>;
    
    /// 检查是否支持某个命令
    fn supports_command(&self, command: Command) -> bool {
        self.supported_commands().contains(&command)
    }
}

/// 组合所有命令处理器
pub struct ServerMessageHandler<S, A, Y> 
where
    S: ServerHandler,
    A: AuthHandler,
    Y: SystemHandler,
{
    auth_handler: AuthCommandHandler<A>,
    server_handler: ServerCommandHandler<S>,
    system_handler: SystemCommandHandler<Y>,
}

impl<S, A, Y> ServerMessageHandler<S, A, Y>
where
    S: ServerHandler,
    A: AuthHandler,
    Y: SystemHandler,
{
    pub fn new(
        auth_handler: AuthCommandHandler<A>,
        server_handler: ServerCommandHandler<S>,
        system_handler: SystemCommandHandler<Y>,
    ) -> Self {
        Self {
            auth_handler,
            server_handler,
            system_handler,
        }
    }

    /// 根据命令选择合适的处理器
    fn get_handler(&self, command: Command) -> Result<&dyn CommandHandler> {
        if self.auth_handler.supports_command(command) {
            Ok(&self.auth_handler)
        } else if self.server_handler.supports_command(command) {
            Ok(&self.server_handler)
        } else if self.system_handler.supports_command(command) {
            Ok(&self.system_handler)
        } else {
            Err(FlareErr::invalid_command(format!(
                "No handler found for command: {:?}",
                command
            )))
        }
    }
    /// 处理新链接
    pub async fn handle_new_connection(&self, ctx:  &AppContext, conn: &ConnectionInfo) -> Result<Response> {
        self.system_handler.handle_new_connection(ctx, conn).await
    }
    /// 认证
    pub async fn handle_auth(&self, ctx:  &AppContext) -> Result<Response> {
        self.auth_handler.handle_login(ctx).await
    }
  
}

#[async_trait]
impl<S, A, Y> CommandHandler for ServerMessageHandler<S, A, Y>
where
    S: ServerHandler + Send + Sync + 'static,
    A: AuthHandler + Send + Sync + 'static,
    Y: SystemHandler + Send + Sync + 'static,
{
    async fn handle_command(&self, ctx:  &AppContext) -> Result<Response> {
        let command = ctx.command().ok_or_else(|| 
            FlareErr::invalid_command("Missing command"))?;

        debug!("Handling command: {:?}", command);

        // 处理特殊命令
        match command {
            Command::Ping => Ok(Response {
                code: ResCode::Success as i32,
                message: "PONG".into(),
                data: Vec::new(),
            }),
            Command::Pong => Ok(Response {
                code: ResCode::Success as i32,
                message: "PING received".into(),
                data: Vec::new(),
            }),
            _ => {
                // 根据命令选择处理器
                let handler = self.get_handler(command)?;
                handler.handle_command(ctx).await
            }
        }
    }

    fn supported_commands(&self) -> Vec<Command> {
        let mut commands = Vec::new();
        commands.extend(self.auth_handler.supported_commands());
        commands.extend(self.server_handler.supported_commands());
        commands.extend(self.system_handler.supported_commands());
        commands.push(Command::Ping);
        commands.push(Command::Pong);
        commands
    }
}

impl<S, A, Y> Default for ServerMessageHandler<S, A, Y>
where
    S: ServerHandler + Default + Send + Sync + 'static,
    A: AuthHandler + Default + Send + Sync + 'static,
    Y: SystemHandler + Default + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(
            AuthCommandHandler::new(A::default()),
            ServerCommandHandler::new(S::default()),
            SystemCommandHandler::new(Y::default())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::auth_handler::DefAuthHandler;
    use crate::server::server_handler::DefServerHandler;
    use crate::server::sys_handler::DefSystemHandler;
    #[tokio::test]
    async fn test_message_handler() {
        let handler = ServerMessageHandler::<DefServerHandler, DefAuthHandler, DefSystemHandler>::default();
        
        // 测试支持的命令
        let commands = handler.supported_commands();
        assert!(commands.contains(&Command::Login));
        assert!(commands.contains(&Command::ClientSendMessage));
        assert!(commands.contains(&Command::SetBackground));
        assert!(commands.contains(&Command::Ping));
        assert!(commands.contains(&Command::Pong));

        // 测试 Ping 命令
        let ctx = AppContext::default();
        let response = handler.handle_command(&ctx).await.unwrap();
        assert_eq!(response.code, ResCode::Success as i32);
        assert_eq!(response.message, "PONG");
    }
}
