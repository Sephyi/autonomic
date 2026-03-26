//! Container configuration and resource limits.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for a single container (or host process) invocation.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Path to the executable to run.
    pub executable: PathBuf,

    /// Arguments to pass to the executable.
    pub args: Vec<String>,

    /// Working directory inside the container (or on the host).
    pub working_dir: PathBuf,

    /// Environment variables to set. Applied after `env_clear`.
    pub env: HashMap<String, String>,

    /// Environment variable names to explicitly remove (applied after `env` is set).
    pub env_remove: Vec<String>,

    /// Maximum wall-clock time before the process is killed.
    pub timeout: Duration,

    /// Resource limits (memory, CPU).
    pub resources: ResourceLimits,

    /// Optional stdin payload to pipe into the process.
    pub stdin: Option<String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("claude"),
            args: Vec::new(),
            working_dir: PathBuf::from("."),
            env: HashMap::new(),
            env_remove: Vec::new(),
            timeout: Duration::from_secs(300),
            resources: ResourceLimits::default(),
            stdin: None,
        }
    }
}

/// Resource limits applied to a container or process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (default: 4 GiB).
    pub memory_bytes: u64,

    /// Number of CPUs (default: 2.0).
    pub cpus: f64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GiB
            cpus: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resource_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_bytes, 4 * 1024 * 1024 * 1024);
        assert!((limits.cpus - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resource_limits_serde_roundtrip() {
        let limits = ResourceLimits {
            memory_bytes: 8 * 1024 * 1024 * 1024,
            cpus: 4.0,
        };
        let json = serde_json::to_string(&limits).expect("serialize");
        let restored: ResourceLimits = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(limits, restored);
    }
}
