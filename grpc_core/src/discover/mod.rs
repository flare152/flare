pub mod consul;
pub mod registry;

pub use registry::{Registry, Registration};
pub use consul::ConsulDiscover;

// Re-export volo's Discover trait and types
pub use volo::discovery::{Discover, Instance, Change};