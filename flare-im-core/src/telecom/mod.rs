#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;

// 客户端导出
#[cfg(feature = "client")]
pub use self::client::{FlareClient, FlareClientBuilder, ConnectionInfo, Protocol};

// 服务端导出
#[cfg(feature = "server")]
pub use self::server::{FlareServer, FlareServerBuilder};