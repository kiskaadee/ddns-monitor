use thiserror::Error;

/// Errors arising during WAN IP resolution.
#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("All configured IP discovery providers failed: {0}")]
    AllProvidersFailed(String),

    #[error("No IP discovery providers configured")]
    NoProvidersConfigured,

    #[error("Invalid IP returned by provider '{provider}': {raw_ip}")]
    InvalidIp { provider: String, raw_ip: String },

    #[error("HTTP request to '{provider}' failed: {details}")]
    HttpError { provider: String, details: String },
}

/// Errors arising during DDNS provider updates.
#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("DDNS provider '{provider}' rejected update: {message}")]
    ProviderRejected { provider: String, message: String },

    #[error("Authentication failed for provider '{provider}': {details}")]
    AuthError { provider: String, details: String },

    #[error("Network error updating provider '{provider}': {details}")]
    NetworkError { provider: String, details: String },

    #[error("Misconfiguration for provider '{provider}': {details}")]
    ConfigurationError { provider: String, details: String },
}

/// Errors arising from state repository persistence.
#[derive(Debug, Error)]
pub enum StateError {
    #[error("I/O error accessing state file '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse state JSON: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Errors arising from history log persistence.
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("I/O error writing history log '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to serialize history entry: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Top-level configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error reading config file '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse TOML configuration: {0}")]
    ParseToml(#[from] toml::de::Error),

    #[error("Missing required environment variable '{var}': {source}")]
    MissingEnvVar {
        var: String,
        #[source]
        source: std::env::VarError,
    },

    #[error("Validation error: {0}")]
    Validation(String),
}
