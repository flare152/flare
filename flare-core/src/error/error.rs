
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use crate::flare_net::net::{ResCode, Response};

pub type Result<T> = std::result::Result<T, FlareErr>;


#[derive(Debug, Error)]
pub enum FlareErr {
    #[error("error: {0}")]
    Error(String),

    #[error("invalid params `{0}`")]
    InvalidParams(String),

    #[error("connection error: {0}")]
    ConnectionError(String),

    #[error("business error: {0}")]
    BusinessError(String),

    #[error("args error: {0}")]
    ArgsError(String),

    #[error("Connection closed")]
    ConnectionClosed,

    /// 链接不存在
    #[error("connection not found")]
    ConnectionNotFound,

    #[error("Failed to decode message: {0}")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Failed to encode message: {0}")]
    EncodeError(#[from] prost::EncodeError),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Invalid message type")]
    InvalidMessageType,

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),

    // 业务相关错误
    #[error("not fond handler")]
    NotFondHandler,

    #[error("push to client error `{0}`")]
    PushToClientErr(String),

    #[error("send message err  code `{0}` msg `{1}`")]
    SendMsgErr(i32, String),

    // 命令相关错误
    #[error("invalid command: {0}")]
    InvalidCommand(String),

    // 认证相关错误
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    // 内部错误
    #[error("internal error: {0}")]
    InternalError(String),

    // 状态错误
    #[error("invalid state: {0}")]
    InvalidState(String),

    // 超时错误
    #[error("timeout: {0}")]
    Timeout(String),

    // 资源错误
    #[error("resource error: {0}")]
    ResourceError(String),

    #[error("service not found: {0}")]
    ServiceNotFound(String),

}
// 推送到客户端产生错误
pub struct FlareError {
    pub code: i32,
    pub err: FlareErr,
}

impl FlareErr {
    // 获取错误码
    pub fn code(&self) -> ResCode {
        match self {
            FlareErr::Error(_) => ResCode::UnknownCode,
            FlareErr::ConnectionError(_) => ResCode::ConnectionError,
            FlareErr::BusinessError(_) => ResCode::BusinessError,
            FlareErr::ArgsError(_) => ResCode::ArgsError,
            FlareErr::InvalidParams(_) => ResCode::InvalidParams,
            FlareErr::ConnectionClosed => ResCode::ConnectionClosed,
            FlareErr::ConnectionNotFound => ResCode::ConnectionNotFound,
            FlareErr::DecodeError(_) => ResCode::DecodeError,
            FlareErr::EncodeError(_) => ResCode::EncodeError,
            _ => ResCode::UnknownCode,
        }
    }

    // 转换为响应
    pub fn to_res(&self) -> Response {
        Response {
            code: self.code() as i32,
            message: self.to_string(),
            data: Vec::new(),
        }
    }

    // 错误转换辅助方法
    pub fn from_str(s: impl Into<String>) -> Self {
        FlareErr::Error(s.into())
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        FlareErr::InvalidParams(msg.into())
    }

    pub fn invalid_command(msg: impl Into<String>) -> Self {
        FlareErr::InvalidCommand(msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        FlareErr::Unauthorized(msg.into())
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        FlareErr::InternalError(msg.into())
    }

    pub fn invalid_state(msg: impl Into<String>) -> Self {
        FlareErr::InvalidState(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        FlareErr::Timeout(msg.into())
    }

    pub fn resource_error(msg: impl Into<String>) -> Self {
        FlareErr::ResourceError(msg.into())
    }

    pub fn not_found_service(msg: impl Into<String>) -> Self {
        FlareErr::ServiceNotFound(msg.into())
    }

    pub fn connection_error(msg: impl Into<String>) -> Self {
        FlareErr::ConnectionError(msg.into())
    }

    pub fn decode_error(err: prost::DecodeError) -> Self {
        FlareErr::DecodeError(err)
    }
}

// 实现 From trait 用于错误转换
impl From<String> for FlareErr {
    fn from(s: String) -> Self {
        FlareErr::Error(s)
    }
}

impl From<&str> for FlareErr {
    fn from(s: &str) -> Self {
        FlareErr::Error(s.to_string())
    }
}

impl From<tungstenite::Error> for FlareErr {
    fn from(err: tungstenite::Error) -> Self {
        FlareErr::WebSocketError(err.to_string())
    }
}

impl From<FlareErr> for ResCode {
    fn from(err: FlareErr) -> Self {
        match err {
            FlareErr::Error(_) => ResCode::UnknownCode,
            FlareErr::ConnectionError(_) => ResCode::ConnectionError,
            FlareErr::BusinessError(_) => ResCode::BusinessError,
            FlareErr::ArgsError(_) => ResCode::ArgsError,
            FlareErr::InvalidParams(_) => ResCode::InvalidParams,
            FlareErr::ConnectionClosed => ResCode::ConnectionClosed,
            FlareErr::ConnectionNotFound => ResCode::ConnectionNotFound,
            FlareErr::DecodeError(_) => ResCode::DecodeError,
            FlareErr::EncodeError(_) => ResCode::EncodeError,
            _ => ResCode::UnknownCode,
        }
    }
}
