pub mod connection;
pub mod error;
pub mod ws;

pub use connection::{Connection, ConnectionState};
pub use error::{ConnectionError, Result};
