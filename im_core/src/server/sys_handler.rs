use crate::common::ctx::AppContext;
use crate::common::ctx::Context;
use crate::common::error::error::{FlareErr, Result};
use crate::server::handlers::CommandHandler;
use crate::server::server::ConnectionInfo;
use async_trait::async_trait;
use log::debug;
use protobuf_codegen::{Command, ResCode, Response};

#[async_trait]
pub trait SystemHandler: Send + Sync {
    /// 处理新链接
    async fn handle_new_connection(&self, ctx: &AppContext,conn : &ConnectionInfo) -> Result<Response>;
    /// 设置后台运行
    async fn handle_set_background(&self, ctx: &AppContext, background: bool) -> Result<Response>;
    /// 设置语言
    async fn handle_set_language(&self, ctx: &AppContext, language: String) -> Result<Response>;
    /// 关闭
    async fn handle_close(&self, ctx: &AppContext) -> Result<Response>;
}

/// 系统命令处理器
pub struct SystemCommandHandler<T>(pub T);

impl<T> SystemCommandHandler<T> {
    pub fn new(handler: T) -> Self {
        Self(handler)
    }
}

// 实现 SystemHandler
#[async_trait]
impl<T: SystemHandler + Send + Sync> SystemHandler for SystemCommandHandler<T> {
    async fn handle_new_connection(&self, ctx: &AppContext, conn: &ConnectionInfo) -> Result<Response> {
        self.0.handle_new_connection(ctx, conn).await
    }

    async fn handle_set_background(&self, ctx: &AppContext, background: bool) -> Result<Response> {
        self.0.handle_set_background(ctx, background).await
    }

    async fn handle_set_language(&self, ctx: &AppContext, language: String) -> Result<Response> {
        self.0.handle_set_language(ctx, language).await
    }
    async fn handle_close(&self, ctx: &AppContext) -> Result<Response> {
        self.0.handle_close(ctx).await
    }
}

#[async_trait]
impl<T: SystemHandler + Send + Sync> CommandHandler for SystemCommandHandler<T> {
    async fn handle_command(&self, ctx: &AppContext) -> Result<Response> {
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
            Command::SetBackground => {
                let background = ctx.bool_data()?;
                self.handle_set_background(ctx, background).await
            }
            Command::SetLanguage => {
                let language = ctx.string_data()?;
                self.handle_set_language(ctx, language).await
            }
            Command::Close => self.handle_close(ctx).await,
            _ => Ok(Response {
                code: ResCode::InvalidCommand as i32,
                message: format!("Unexpected command: {:?}", command),
                data: Vec::new(),
            })
        }
    }

    fn supported_commands(&self) -> Vec<Command> {
        vec![Command::SetBackground, Command::SetLanguage, Command::Close]
    }
}

/// 默认的系统处理器实现
#[derive(Default)]
pub struct DefSystemHandler;

impl DefSystemHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SystemHandler for DefSystemHandler {
    async fn handle_new_connection(&self, ctx: &AppContext, conn: &ConnectionInfo) -> Result<Response> {
        debug!("处理新连接请求 - addr: {}, conn_id: {}", ctx.remote_addr(), conn.get_conn_id());
        Ok(Response {
            code: ResCode::Success as i32,
            message: "连接已建立".into(),
            data: Vec::new(),
        })
    }
    
    async fn handle_set_background(&self, ctx: &AppContext, background: bool) -> Result<Response> {
        debug!("处理设置后台运行请求 - addr: {}, background: {}", ctx.remote_addr(), background);

        // 这里可以添加实际的后台运行设置逻辑
        Ok(Response {
            code: ResCode::Success as i32,
            message: format!("后台运行已{}",if background {"开启"} else {"关闭"}),
            data: Vec::new(),
        })
    }

    async fn handle_set_language(&self, ctx: &AppContext, language: String) -> Result<Response> {
        debug!("处理设置语言请求 - addr: {}, language: {}", ctx.remote_addr(), language);

        // 这里可以添加语言设置验证逻辑
        if language.is_empty() {
            return Ok(Response {
                code: ResCode::InvalidParams as i32,
                message: "Language cannot be empty".into(),
                data: Vec::new(),
            });
        }

        Ok(Response {
            code: ResCode::Success as i32,
            message: format!("语言已设置为 {}", language),
            data: Vec::new(),
        })
    }

    async fn handle_close(&self, ctx: &AppContext) -> Result<Response> {
        debug!("处理关闭连接请求 - addr: {}", ctx.remote_addr());
        
        if let Some(user_id) = ctx.user_id() {
            debug!("用户 {} 请求关闭连接", user_id);
        }

        Ok(Response {
            code: ResCode::Success as i32,
            message: "连接即将关闭".into(),
            data: Vec::new(),
        })
    }
}
