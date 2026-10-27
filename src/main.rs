use std::path::{Path, PathBuf};
use std::time::Duration;

use ddns_monitor::config::AppConfig;
use ddns_monitor::domain::traits::DdnsUpdater;
use ddns_monitor::infra::history_store::JsonlHistoryRepository;
use ddns_monitor::infra::http_resolver::HttpIpResolver;
use ddns_monitor::infra::state_store::JsonStateRepository;
use ddns_monitor::plugins::dynu::DynuUpdater;
use ddns_monitor::service::{IpMonitorService, RunOutcome};

fn find_config_path() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "-c" || args[i] == "--config") && i + 1 < args.len() {
            return Ok(PathBuf::from(&args[i + 1]));
        }
        i += 1;
    }

    if let Ok(env_path) = std::env::var("DDNS_CONFIG_PATH") {
        return Ok(PathBuf::from(env_path));
    }

    let candidate = Path::new("config.toml");
    if candidate.exists() {
        return Ok(candidate.to_path_buf());
    }

    if let Ok(home) = std::env::var("HOME") {
        let xdg = PathBuf::from(home)
            .join(".config")
            .join("ddns-monitor")
            .join("config.toml");
        if xdg.exists() {
            return Ok(xdg);
        }
    }

    let sys = Path::new("/etc/ddns-monitor/config.toml");
    if sys.exists() {
        return Ok(sys.to_path_buf());
    }

    Err("No configuration file found. Specify via --config, DDNS_CONFIG_PATH, or ~/.config/ddns-monitor/config.toml".to_string())
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    log::info!("ddns-monitor starting");

    let config_path = match find_config_path() {
        Ok(path) => path,
        Err(e) => {
            log::error!("{}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    log::info!("Loading configuration from '{}'", config_path.display());
    let config = match AppConfig::load_from_path(&config_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to load configuration: {}", e);
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let timeout = Duration::from_secs(config.ip_providers.timeout_secs);
    let resolver = HttpIpResolver::new(config.ip_providers.urls.clone(), timeout);
    let state_repo = JsonStateRepository::new(config.resolve_state_path());
    let history_repo = JsonlHistoryRepository::new(config.resolve_history_path());

    let mut updaters: Vec<Box<dyn DdnsUpdater>> = Vec::new();
    for sub in &config.subscribers {
        match sub.provider.as_str() {
            "dynu" => {
                let credential = match sub.resolve_credential() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed resolving credentials for '{}': {}", sub.domain, e);
                        eprintln!("Credential error for {}: {}", sub.domain, e);
                        std::process::exit(1);
                    }
                };
                updaters.push(Box::new(DynuUpdater::new(&sub.domain, &credential, timeout)));
            }
            unknown => {
                log::error!("Unsupported DDNS provider: '{}'", unknown);
                eprintln!("Unsupported provider: {}", unknown);
                std::process::exit(1);
            }
        }
    }

    let service = IpMonitorService::new(
        Box::new(resolver),
        Box::new(state_repo),
        Box::new(history_repo),
        updaters,
    );

    match service.run() {
        Ok(RunOutcome::NoChange { ip }) => {
            log::info!("Execution complete: No IP change detected (current: {}).", ip);
            std::process::exit(0);
        }
        Ok(RunOutcome::Updated { ip, subscribers }) => {
            log::info!(
                "Execution complete: Successfully updated {} subscriber(s) with IP {}.",
                subscribers.len(),
                ip
            );
            std::process::exit(0);
        }
        Ok(RunOutcome::PartialFailure { ip, subscribers }) => {
            log::error!(
                "Execution finished with partial failure: IP is {}, but not all subscribers succeeded: {:?}",
                ip,
                subscribers
            );
            std::process::exit(1);
        }
        Ok(RunOutcome::AllFailed { ip, subscribers }) => {
            log::error!(
                "Execution failed: IP is {}, but all subscribers failed: {:?}",
                ip,
                subscribers
            );
            std::process::exit(1);
        }
        Err(e) => {
            log::error!("Service error: {}", e);
            eprintln!("Service error: {}", e);
            std::process::exit(1);
        }
    }
}
