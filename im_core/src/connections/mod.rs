mod connection;
pub use connection::{Connection, ConnectionState};

#[cfg(any(feature = "client", feature = "server"))]
pub mod ws;

#[cfg(any(feature = "client", feature = "server"))]
pub mod quic;

#[cfg(any(feature = "client", feature = "server"))]
pub use ws::WsConnection;

#[cfg(any(feature = "client", feature = "server"))]
pub use quic::QuicConnection;

// 导出 quic 配置
#[cfg(any(feature = "client", feature = "server"))]
pub mod quic_conf {
    pub use super::quic::config::*;
}