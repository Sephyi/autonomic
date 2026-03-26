//! Podman/Docker container runtime abstraction for Autonomic.
//!
//! Phase 1: `HostRuntime` spawns Claude Code directly on the host via
//! `tokio::process::Command`. The `ContainerRuntime` trait exists so
//! `PodmanRuntime` can be added in Phase 2 without changing callers.

pub mod config;
pub mod error;
pub mod host;
pub mod runtime;

pub use config::{ContainerConfig, ResourceLimits};
pub use error::ContainerError;
pub use host::HostRuntime;
pub use runtime::{ContainerRuntime, ProcessOutput};
