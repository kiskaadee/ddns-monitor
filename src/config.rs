use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::errors::ConfigError;

/// Configuration for the public IP discovery checkers.
#[derive(Debug, Clone, Deserialize)]
pub struct IpProvidersConfig {
    /// List of external IP-echo URLs (e.g. "https://api.ipify.org").
    pub urls: Vec<String>,

    /// Timeout in seconds per HTTP check attempt.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_timeout_secs() -> u64 {
    5
}

/// Configuration for an individual DDNS subscriber.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriberConfig {
    /// Provider type ("dynu", etc.).
    pub provider: String,

    /// Domain name to update.
    pub domain: String,

    /// Environment variable name holding the credential or API key.
    pub credentials_env: Option<String>,

    /// Path to a secret file containing the credential.
    pub credentials_file: Option<PathBuf>,
}

impl SubscriberConfig {
    /// Resolves the credential for this subscriber from the environment
    /// or from a file.
    pub fn resolve_credential(&self) -> Result<String, ConfigError> {
        if let Some(ref env_var) = self.credentials_env {
            match std::env::var(env_var) {
                Ok(val) => {
                    let trimmed = val.trim().to_string();
                    if !trimmed.is_empty() {
                        return Ok(trimmed);
                    }
                }
                Err(e) => {
                    if self.credentials_file.is_none() {
                        return Err(ConfigError::MissingEnvVar {
                            var: env_var.clone(),
                            source: e,
                        });
                    }
                }
            }
        }

        if let Some(ref file_path) = self.credentials_file {
            let content = fs::read_to_string(file_path).map_err(|source| ConfigError::Io {
                path: file_path.to_string_lossy().to_string(),
                source,
            })?;
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "Credential file '{}' is empty",
                    file_path.display()
                )));
            }
            return Ok(trimmed);
        }

        Err(ConfigError::Validation(format!(
            "Subscriber for domain '{}' must specify either 'credentials_env' or 'credentials_file'",
            self.domain
        )))
    }
}

/// Root application configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// IP provider settings.
    pub ip_providers: IpProvidersConfig,

    /// List of configured DDNS subscribers.
    #[serde(default)]
    pub subscribers: Vec<SubscriberConfig>,

    /// Optional explicit path to the state file.
    pub state_path: Option<PathBuf>,

    /// Optional explicit path to the JSONL history audit log.
    pub history_path: Option<PathBuf>,
}

impl AppConfig {
    /// Deserializes and validates configuration from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        let config: AppConfig = toml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }

    /// Loads and validates configuration from a file on disk.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref).map_err(|source| ConfigError::Io {
            path: path_ref.to_string_lossy().to_string(),
            source,
        })?;
        Self::from_toml(&content)
    }

    /// Validates the configuration semantics.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.ip_providers.urls.is_empty() {
            return Err(ConfigError::Validation(
                "At least one IP provider URL must be configured".to_string(),
            ));
        }

        if self.ip_providers.timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "IP provider timeout must be greater than 0 seconds".to_string(),
            ));
        }

        for sub in &self.subscribers {
            if sub.domain.trim().is_empty() {
                return Err(ConfigError::Validation(
                    "Subscriber domain cannot be empty".to_string(),
                ));
            }
            if sub.credentials_env.is_none() && sub.credentials_file.is_none() {
                return Err(ConfigError::Validation(format!(
                    "Subscriber for domain '{}' must have credentials_env or credentials_file",
                    sub.domain
                )));
            }
        }

        Ok(())
    }

    /// Returns the resolved state file path (config override -> systemd StateDirectory -> XDG).
    pub fn resolve_state_path(&self) -> PathBuf {
        if let Some(ref path) = self.state_path {
            return path.clone();
        }

        if let Ok(state_dir) = std::env::var("STATE_DIRECTORY") {
            return PathBuf::from(state_dir).join("state.json");
        }

        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("ddns-monitor")
                .join("state.json");
        }

        PathBuf::from("/var/lib/ddns-monitor/state.json")
    }

    /// Returns the resolved history file path (config override -> systemd StateDirectory -> XDG).
    pub fn resolve_history_path(&self) -> PathBuf {
        if let Some(ref path) = self.history_path {
            return path.clone();
        }

        if let Ok(state_dir) = std::env::var("STATE_DIRECTORY") {
            return PathBuf::from(state_dir).join("history.jsonl");
        }

        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("ddns-monitor")
                .join("history.jsonl");
        }

        PathBuf::from("/var/lib/ddns-monitor/history.jsonl")
    }
}
