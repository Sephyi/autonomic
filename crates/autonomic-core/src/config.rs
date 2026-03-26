//! Configuration types and loading for Autonomic.
//!
//! Config is loaded via figment: TOML file + environment variable overrides.
//! Secrets (API keys, database URL) are in a separate secrets.toml (XD-011).

use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::CoreError;
use crate::rate_budget::BudgetAllocation;

/// Top-level Autonomic configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicConfig {
    /// Base directory for Autonomic state. Default: ~/.autonomic
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,

    /// HTTP API configuration.
    #[serde(default)]
    pub api: ApiConfig,

    /// Rate budget configuration.
    #[serde(default)]
    pub budget: BudgetConfig,

    /// Session defaults.
    #[serde(default)]
    pub session: SessionDefaults,
}

fn default_state_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".autonomic")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Localhost port for the daemon HTTP API.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Bind address.
    #[serde(default = "default_bind")]
    pub bind: String,
}

fn default_port() -> u16 {
    7700
}

fn default_bind() -> String {
    "127.0.0.1".into()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            bind: default_bind(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum budget in USD for the 5-hour sliding window.
    #[serde(default = "default_global_budget")]
    pub global_budget_usd: f64,

    /// Budget allocation percentages.
    #[serde(default)]
    pub allocation: BudgetAllocation,
}

fn default_global_budget() -> f64 {
    50.0
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            global_budget_usd: default_global_budget(),
            allocation: BudgetAllocation::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDefaults {
    /// Default timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Default cost budget per session in USD.
    #[serde(default = "default_cost_budget")]
    pub cost_budget_usd: f64,

    /// Maximum concurrent sessions.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,

    /// Override path to the Claude Code binary. If unset, searches `$PATH`.
    #[serde(default)]
    pub claude_binary: Option<PathBuf>,
}

fn default_timeout() -> u64 {
    1800
}

fn default_cost_budget() -> f64 {
    5.0
}

fn default_max_concurrent() -> usize {
    5
}

impl Default for SessionDefaults {
    fn default() -> Self {
        Self {
            timeout_seconds: default_timeout(),
            cost_budget_usd: default_cost_budget(),
            max_concurrent: default_max_concurrent(),
            claude_binary: None,
        }
    }
}

/// Secrets loaded from secrets.toml (gitignored, per XD-011).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Database connection details.
    #[serde(default)]
    pub database: DatabaseSecrets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSecrets {
    /// PostgreSQL connection URL.
    #[serde(default = "default_database_url")]
    pub url: String,
}

fn default_database_url() -> String {
    "postgresql://autonomic:autonomic_dev_password@localhost:5432/autonomic".into()
}

impl Default for DatabaseSecrets {
    fn default() -> Self {
        Self {
            url: default_database_url(),
        }
    }
}

/// Load the main config from a TOML file.
pub fn load_config(config_path: &Path) -> Result<AutonomicConfig, CoreError> {
    if !config_path.exists() {
        return Ok(AutonomicConfig {
            state_dir: config_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
            api: ApiConfig::default(),
            budget: BudgetConfig::default(),
            session: SessionDefaults::default(),
        });
    }

    let config: AutonomicConfig = Figment::new()
        .merge(Toml::file(config_path))
        .extract()
        .map_err(|e| CoreError::Config(e.to_string()))?;

    Ok(config)
}

/// Load secrets from secrets.toml (per XD-011: secrets from file, not env).
pub fn load_secrets(secrets_path: &Path) -> Result<SecretsConfig, CoreError> {
    if !secrets_path.exists() {
        return Ok(SecretsConfig::default());
    }

    let secrets: SecretsConfig = Figment::new()
        .merge(Toml::file(secrets_path))
        .extract()
        .map_err(|e| CoreError::Config(e.to_string()))?;

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_when_file_missing() {
        let config = load_config(Path::new("/nonexistent/config.toml")).unwrap();
        assert_eq!(config.api.port, 7700);
        assert_eq!(config.session.timeout_seconds, 1800);
    }

    #[test]
    fn load_config_from_toml() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
[api]
port = 8800

[session]
timeout_seconds = 900
"#
        )
        .unwrap();

        let config = load_config(f.path()).unwrap();
        assert_eq!(config.api.port, 8800);
        assert_eq!(config.session.timeout_seconds, 900);
    }

    #[test]
    fn default_secrets_when_file_missing() {
        let secrets = load_secrets(Path::new("/nonexistent/secrets.toml")).unwrap();
        assert!(secrets.database.url.contains("localhost:5432"));
    }

    #[test]
    fn default_config_snapshot() {
        let config = load_config(Path::new("/nonexistent/config.toml")).unwrap();
        // Snapshot the serialized TOML to catch unintended default changes
        let serialized = toml::to_string_pretty(&config).unwrap();
        insta::assert_snapshot!("default_config", serialized);
    }
}
