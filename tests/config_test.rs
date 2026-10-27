use ddns_monitor::config::{AppConfig, SubscriberConfig};
use ddns_monitor::domain::errors::ConfigError;
use std::fs;

#[test]
fn test_parse_valid_toml_config() {
    let toml_data = r#"
[ip_providers]
urls = [
    "https://api.ipify.org",
    "https://ifconfig.me/ip"
]
timeout_secs = 5

[[subscribers]]
provider = "dynu"
domain = "example.dyndns.org"
credentials_env = "TEST_DYNU_SECRET"

[[subscribers]]
provider = "dynu"
domain = "backup.dyndns.org"
credentials_file = "/tmp/fake_secret"
"#;

    let config = AppConfig::from_toml(toml_data).expect("valid config");
    assert_eq!(config.ip_providers.urls.len(), 2);
    assert_eq!(config.ip_providers.timeout_secs, 5);
    assert_eq!(config.subscribers.len(), 2);
    assert_eq!(config.subscribers[0].provider, "dynu");
    assert_eq!(config.subscribers[0].domain, "example.dyndns.org");
    assert_eq!(
        config.subscribers[0].credentials_env.as_deref(),
        Some("TEST_DYNU_SECRET")
    );
}

#[test]
fn test_resolve_credential_from_env() {
    let sub = SubscriberConfig {
        provider: "dynu".to_string(),
        domain: "test.org".to_string(),
        credentials_env: Some("DDNS_TEST_ENV_VAR_123".to_string()),
        credentials_file: None,
    };

    // Before setting env var
    unsafe {
        std::env::remove_var("DDNS_TEST_ENV_VAR_123");
    }
    assert!(sub.resolve_credential().is_err());

    // After setting env var
    unsafe {
        std::env::set_var("DDNS_TEST_ENV_VAR_123", "secret123");
    }
    let secret = sub.resolve_credential().expect("resolved from env");
    assert_eq!(secret, "secret123");
    unsafe {
        std::env::remove_var("DDNS_TEST_ENV_VAR_123");
    }
}

#[test]
fn test_resolve_credential_from_file() {
    let temp_path = std::env::temp_dir().join("ddns_monitor_test_secret.txt");
    fs::write(&temp_path, "  my-file-secret \n").unwrap();

    let sub = SubscriberConfig {
        provider: "dynu".to_string(),
        domain: "test.org".to_string(),
        credentials_env: None,
        credentials_file: Some(temp_path.clone()),
    };

    let secret = sub.resolve_credential().expect("resolved from file");
    assert_eq!(secret, "my-file-secret");
    let _ = fs::remove_file(temp_path);
}

#[test]
fn test_empty_urls_validation_error() {
    let toml_data = r#"
[ip_providers]
urls = []
timeout_secs = 5

[[subscribers]]
provider = "dynu"
domain = "example.com"
credentials_env = "FOO"
"#;

    let res = AppConfig::from_toml(toml_data);
    assert!(res.is_err());
    match res.unwrap_err() {
        ConfigError::Validation(msg) => {
            assert!(msg.contains("At least one IP provider URL"));
        }
        err => panic!("Unexpected error type: {:?}", err),
    }
}
