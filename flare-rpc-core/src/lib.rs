#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod app;

pub mod discover;
pub mod interceptor;

#[cfg(feature = "etcd")]
extern crate etcd_client;

#[cfg(feature = "client")]
pub use client::*;

#[cfg(feature = "server")]
pub use app::*;


