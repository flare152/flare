mod consul;
mod registry;
mod etcd;
mod discover;

pub use discover::{RpcDiscovery};
pub use registry::{Registry, Registration,LogRegistry};
pub use consul::{ConsulDiscover, ConsulConfig,ConsulRegistry};
pub use etcd::{EtcdDiscover, EtcdConfig, EtcdRegistry};

// Re-export volo's Discover trait and types
pub use volo::discovery::{Discover, Instance, Change};