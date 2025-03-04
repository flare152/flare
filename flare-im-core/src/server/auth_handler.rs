use flare_core::context::AppContext;
use flare_core::error::{FlareErr, Result};
use crate::server::handlers::CommandHandler;
use async_trait::async_trait;
use log::debug;
use prost::Message;
use flare_core::flare_net::net::{LoginReq, LoginResp};
use flare_core::flare_net::net::{Command, ResCode, Response};

#[async_trait]
pub trait AuthHandler: Send + Sync {
    /// 处理登录请求
    async fn handle_login(&self, ctx:  &AppContext) -> Result<Response>;

    /// 处理登出请求
    async fn handle_logout(&self, ctx:  &AppContext) -> Result<Response>;
}

/// 认证命令处理器
pub struct AuthCommandHandler<T>(pub T);

impl<T> AuthCommandHandler<T> {
    pub fn new(handler: T) -> Self {
        Self(handler)
    }
}

// 实现 AuthHandler
#[async_trait]
impl<T: AuthHandler + Send + Sync> AuthHandler for AuthCommandHandler<T> {
    async fn handle_login(&self, ctx:  &AppContext) -> Result<Response> {
        self.0.handle_login(ctx).await
    }

    async fn handle_logout(&self, ctx:  &AppContext) -> Result<Response> {
        self.0.handle_logout(ctx).await
    }
}

#[async_trait]
impl<T: AuthHandler + Send + Sync> CommandHandler for AuthCommandHandler<T> {
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
            Command::Login => self.handle_login(ctx).await,
            Command::LoginOut => self.handle_logout(ctx).await,
            _ => Ok(Response {
                code: ResCode::InvalidCommand as i32,
                message: format!("Unexpected command: {:?}", command),
                data: Vec::new(),
            })
        }
    }
    fn supported_commands(&self) -> Vec<Command> {
        vec![Command::Login , Command::LoginOut]
    }
}

/// 默认的认证处理器实现
#[derive(Default)]
pub struct DefAuthHandler;

impl DefAuthHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AuthHandler for DefAuthHandler {
    async fn handle_login(&self, ctx:  &AppContext) -> Result<Response> {
        let req = ctx.get_data_as::<LoginReq>()?;
        debug!("处理登录请求 - addr: {}, userid: {}", ctx.remote_addr(), req.user_id);

        // 这里可以添加实际的登录验证逻辑
        if req.token.is_empty() {
            return Ok(Response {
                code: ResCode::Unauthorized as i32,
                message: "Token is required".into(),
                data: Vec::new(),
            });
        }
        let resp = LoginResp {
            user_id:"sss".to_string(),
            language:"zh".to_string(),
        };
        Ok(Response {
            code: ResCode::Success as i32,
            message: "登录成功".into(),
            data: resp.encode_to_vec(),
        })
    }

    async fn handle_logout(&self, ctx:  &AppContext) -> Result<Response> {
        debug!("处理登出请求 - addr: {}", ctx.remote_addr());
        
        if let Some(user_id) = ctx.user_id() {
            debug!("用户 {} 登出", user_id);
        }

        Ok(Response {
            code: ResCode::Success as i32,
            message: "登出成功".into(),
            data: Vec::new(),
        })
    }
}
