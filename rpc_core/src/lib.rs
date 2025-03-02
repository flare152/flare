
mod discover;
mod app;

pub use discover::{
    Registration, Registry,
    RpcDiscovery,
    EtcdConfig, EtcdDiscover, EtcdRegistry,
    ConsulConfig, ConsulDiscover, ConsulRegistry,
};

extern crate etcd_client;

