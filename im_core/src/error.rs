use protobuf_codegen::ResCode;
use thiserror::Error;
use tokio_tungstenite::tungstenite;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Connection timeout")]
    Timeout,

    #[error("Failed to decode message: {0}")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Failed to encode message: {0}")]
    EncodeError(#[from] prost::EncodeError),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Invalid message type")]
    InvalidMessageType,

    #[error("Business error: code={code:?}, message={message}")]
    BusinessError {
        code: ResCode,
        message: String,
    },

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<tungstenite::Error> for ConnectionError {
    fn from(err: tungstenite::Error) -> Self {
        ConnectionError::WebSocketError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ConnectionError>; 