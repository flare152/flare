use flare::context::AppContext;
use flare::error::{FlareErr, Result};
use crate::server::handlers::CommandHandler;
use async_trait::async_trait;
use log::debug;
use protobuf_codegen::{Command, ResCode, Response};

/// 服务端处理器
#[async_trait]
pub trait ServerHandler: Send + Sync {
    /// 处理发送消息
    async fn handle_send_message(&self, ctx:  &AppContext) -> Result<Response>;

    /// 处理拉取消息
    async fn handle_pull_message(&self, ctx:  &AppContext) -> Result<Response>;

    /// 处理数据请求
    async fn handle_request(&self, ctx:  &AppContext) -> Result<Response>;

    /// 处理消息ack
    async fn handle_ack(&self, ctx:  &AppContext) -> Result<Response>;
}

/// 服务端命令处理器
pub struct ServerCommandHandler<T>(pub T);

impl<T> ServerCommandHandler<T> {
    pub fn new(handler: T) -> Self {
        Self(handler)
    }
}

// 实现 ServerHandler
#[async_trait]
impl<T: ServerHandler + Send + Sync> ServerHandler for ServerCommandHandler<T> {
    async fn handle_send_message(&self, ctx:  &AppContext) -> Result<Response> {
        self.0.handle_send_message(ctx).await
    }

    async fn handle_pull_message(&self, ctx:  &AppContext) -> Result<Response> {
        self.0.handle_pull_message(ctx).await
    }

    async fn handle_request(&self, ctx:  &AppContext) -> Result<Response> {
        self.0.handle_request(ctx).await
    }

    async fn handle_ack(&self, ctx:  &AppContext) -> Result<Response> {
        self.0.handle_ack(ctx).await
    }
}

#[async_trait]
impl<T: ServerHandler + Send + Sync> CommandHandler for ServerCommandHandler<T> {
    async fn handle_command(&self, ctx:  &AppContext) -> Result<Response> {
        let command = ctx.command().ok_or_else(|| 
            FlareErr::invalid_command("Missing command"))?;

        if !self.supports_command(command) {
            return Ok(Response {
                code: ResCode::InvalidCommand as i32,
                message: format!("Unsupported command: {:?}", command),
                data: Vec::new(),
            });
        }

        match command {
            Command::ClientSendMessage => self.handle_send_message(ctx).await,
            Command::ClientPullMessage => self.handle_pull_message(ctx).await,
            Command::ClientRequest => self.handle_request(ctx).await,
            Command::ClientAck => self.handle_ack(ctx).await,
            _ => Ok(Response {
                code: ResCode::InvalidCommand as i32,
                message: format!("Unexpected command: {:?}", command),
                data: Vec::new(),
            })
        }
    }

    fn supported_commands(&self) -> Vec<Command> {
        vec![
            Command::ClientSendMessage,
            Command::ClientPullMessage,
            Command::ClientRequest,
            Command::ClientAck,
        ]
    }
}

/// 默认的服务端处理器实现
#[derive(Default)]
pub struct DefServerHandler;

impl DefServerHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ServerHandler for DefServerHandler {
    async fn handle_send_message(&self, ctx:  &AppContext) -> Result<Response> {
        debug!("处理发送消息请求 - addr: {}", ctx.remote_addr());

        let message = ctx.string_data()?;
        if message.is_empty() {
            return Ok(Response {
                code: ResCode::InvalidParams as i32,
                message: "Message cannot be empty".into(),
                data: Vec::new(),
            });
        }

        // 这里可以添加实际的消息发送逻辑
        Ok(Response {
            code: ResCode::Success as i32,
            message: "消息发送成功".into(),
            data: Vec::new(),
        })
    }

    async fn handle_pull_message(&self, ctx:  &AppContext) -> Result<Response> {
        debug!("处理拉取消息请求 - addr: {}", ctx.remote_addr());

        // 这里可以添加实际的消息拉取逻辑
        Ok(Response {
            code: ResCode::Success as i32,
            message: "消息拉取成功".into(),
            data: Vec::new(), // 这里应该返回实际的消息数据
        })
    }

    async fn handle_request(&self, ctx:  &AppContext) -> Result<Response> {
        debug!("处理数据请求 - addr: {}", ctx.remote_addr());

        // 这里可以添加实际的数据请求处理逻辑
        Ok(Response {
            code: ResCode::Success as i32,
            message: "请求处理成功".into(),
            data: Vec::new(), // 这里应该返回请求的数据
        })
    }

    async fn handle_ack(&self, ctx:  &AppContext) -> Result<Response> {
        debug!("处理消息确认 - addr: {}", ctx.remote_addr());

        let msg_id = ctx.msg_id()?;
        if msg_id.is_empty() {
            return Ok(Response {
                code: ResCode::InvalidParams as i32,
                message: "Message ID cannot be empty".into(),
                data: Vec::new(),
            });
        }

        // 这里可以添加实际的消息确认处理逻辑
        Ok(Response {
            code: ResCode::Success as i32,
            message: format!("消息 {} 确认成功", msg_id),
            data: Vec::new(),
        })
    }
}
