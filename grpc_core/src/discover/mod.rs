mod consul;
mod registry;
mod etcd;

pub use registry::{Registry, Registration};
pub use consul::ConsulDiscover;
pub use etcd::EtcdDiscover;

// Re-export volo's Discover trait and types
pub use volo::discovery::{Discover, Instance, Change};