//! Core identifier and classification types for Autonomic.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ulid::Ulid;

/// A unique identifier for a session, backed by a ULID.
///
/// ULIDs are time-sortable and URL-safe, making them suitable for use
/// as database primary keys and in log correlation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Ulid);

impl SessionId {
    /// Generate a new `SessionId` from the current timestamp.
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A stable identifier for a project, derived from its filesystem path.
///
/// Computed as the first 16 hex characters (8 bytes) of the SHA-256 hash
/// of the canonical path string. This is stable across Rust versions since
/// SHA-256 is a fixed standard, unlike `DefaultHasher`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(String);

impl ProjectId {
    /// Derive a `ProjectId` from a filesystem path.
    ///
    /// Uses the first 8 bytes (16 hex chars) of the SHA-256 hash of the
    /// path's UTF-8 representation.
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy();
        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        let result = hasher.finalize();
        let hex = hex_encode(&result[..8]);
        Self(hex)
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Encode a byte slice as lowercase hexadecimal.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The Claude model tier to use for a session or task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

impl fmt::Display for ModelTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelTier::Haiku => write!(f, "haiku"),
            ModelTier::Sonnet => write!(f, "sonnet"),
            ModelTier::Opus => write!(f, "opus"),
        }
    }
}

/// The lifecycle status of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// The subsystem that originated or owns a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Subsystem {
    Interactive,
    Scheduled,
    Evolution,
    Monitoring,
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Subsystem::Interactive => write!(f, "interactive"),
            Subsystem::Scheduled => write!(f, "scheduled"),
            Subsystem::Evolution => write!(f, "evolution"),
            Subsystem::Monitoring => write!(f, "monitoring"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_is_unique() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn session_id_display_is_26_chars() {
        let id = SessionId::new();
        // ULIDs are 26 characters in Crockford base32 encoding
        assert_eq!(id.to_string().len(), 26);
    }

    #[test]
    fn project_id_from_path_is_stable() {
        let path = Path::new("/home/user/projects/my-app");
        let id1 = ProjectId::from_path(path);
        let id2 = ProjectId::from_path(path);
        assert_eq!(id1, id2);
    }

    #[test]
    fn project_id_is_16_hex_chars() {
        let path = Path::new("/home/user/projects/my-app");
        let id = ProjectId::from_path(path);
        let s = id.to_string();
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn project_id_differs_for_different_paths() {
        let a = ProjectId::from_path(Path::new("/projects/a"));
        let b = ProjectId::from_path(Path::new("/projects/b"));
        assert_ne!(a, b);
    }

    #[test]
    fn model_tier_display() {
        assert_eq!(ModelTier::Haiku.to_string(), "haiku");
        assert_eq!(ModelTier::Sonnet.to_string(), "sonnet");
        assert_eq!(ModelTier::Opus.to_string(), "opus");
    }

    #[test]
    fn session_status_display() {
        assert_eq!(SessionStatus::Running.to_string(), "running");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
        assert_eq!(SessionStatus::Failed.to_string(), "failed");
        assert_eq!(SessionStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn subsystem_display() {
        assert_eq!(Subsystem::Interactive.to_string(), "interactive");
        assert_eq!(Subsystem::Scheduled.to_string(), "scheduled");
        assert_eq!(Subsystem::Evolution.to_string(), "evolution");
        assert_eq!(Subsystem::Monitoring.to_string(), "monitoring");
    }

    #[test]
    fn serde_roundtrip_model_tier() {
        let tier = ModelTier::Sonnet;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"sonnet\"");
        let decoded: ModelTier = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, tier);
    }

    #[test]
    fn serde_roundtrip_session_id() {
        let id = SessionId::new();
        let json = serde_json::to_string(&id).unwrap();
        let decoded: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, id);
    }
}
