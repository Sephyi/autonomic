//! Container runtime trait and output types.
//!
//! Uses return-position `impl Trait` in trait methods (RPITIT), stabilized
//! in Rust 1.75. This means the trait is *not* object-safe (`dyn ContainerRuntime`
//! is not supported). Use generics (`impl ContainerRuntime`) or an enum dispatch
//! wrapper instead.
//!
//! TODO(phase-2): Add `PodmanRuntime` implementation behind a `podman` feature flag.

use crate::config::ContainerConfig;
use crate::error::ContainerError;

/// Output captured from a completed process.
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// The process exit code (if it exited normally).
    pub exit_code: Option<i32>,

    /// Captured stdout.
    pub stdout: String,

    /// Captured stderr.
    pub stderr: String,
}

/// Abstraction over process execution backends.
///
/// Phase 1 provides [`crate::host::HostRuntime`] (direct subprocess).
/// Phase 2 will add `PodmanRuntime` (OCI container).
///
/// Note: this trait uses RPITIT and is therefore not object-safe.
/// Use `impl ContainerRuntime` or enum dispatch instead of `dyn`.
pub trait ContainerRuntime: Send + Sync {
    /// Spawn a process with the given configuration, wait for completion,
    /// and return captured output.
    fn spawn(
        &self,
        config: &ContainerConfig,
    ) -> impl Future<Output = Result<ProcessOutput, ContainerError>> + Send;

    /// Check whether this runtime backend is available on the current system.
    fn is_available(&self) -> impl Future<Output = bool> + Send;

    /// Human-readable name for this runtime (e.g. "host", "podman").
    fn name(&self) -> &str;
}
