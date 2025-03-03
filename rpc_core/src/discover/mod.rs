#[cfg(feature = "consul")]
pub mod consul;
#[cfg(feature = "etcd")]
mod etcd;
mod discover;

#[cfg(feature = "server")]
mod registry;

#[cfg(feature = "client")]
pub use discover::{RpcDiscovery, LoadBalanceStrategy, LoadBalancer, ServiceEndpoint, ServiceError};

#[cfg(feature = "server")]
pub use registry::{Registry, Registration, LogRegistry};

#[cfg(all(feature = "consul", feature = "client"))]
pub use consul::ConsulDiscover;
#[cfg(all(feature = "consul", feature = "server"))]
pub use consul::{ConsulConfig, ConsulRegistry};

#[cfg(all(feature = "etcd", feature = "client"))]
pub use etcd::EtcdDiscover;
#[cfg(all(feature = "etcd", feature = "server"))]
pub use etcd::{EtcdConfig, EtcdRegistry};

// Re-export tonic's discovery trait and types
#[cfg(feature = "client")]
pub use tonic::transport::channel::Channel;
