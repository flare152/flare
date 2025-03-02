pub mod consul;
mod registry;
mod etcd;
mod discover;

pub use discover::{RpcDiscovery, LoadBalanceStrategy, LoadBalancer, ServiceEndpoint,ServiceError};
pub use registry::{Registry, Registration,LogRegistry};
pub use consul::{ConsulDiscover, ConsulConfig,ConsulRegistry};
pub use etcd::{EtcdDiscover, EtcdConfig, EtcdRegistry};

// Re-export tonic's discovery trait and types
pub use tonic::transport::channel::Channel;
