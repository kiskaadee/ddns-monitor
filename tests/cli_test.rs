use ddns_monitor::config::AppConfig;
use ddns_monitor::domain::traits::DdnsUpdater;
use ddns_monitor::infra::history_store::JsonlHistoryRepository;
use ddns_monitor::infra::http_resolver::HttpIpResolver;
use ddns_monitor::infra::state_store::JsonStateRepository;
use ddns_monitor::plugins::dynu::DynuUpdater;
use ddns_monitor::service::{IpMonitorService, RunOutcome};
use std::fs;
use std::net::Ipv4Addr;
use std::time::Duration;

#[test]
fn test_end_to_end_wireup() {
    let mut ip_server = mockito::Server::new();
    let _m_ip = ip_server
        .mock("GET", "/ip")
        .with_status(200)
        .with_body("93.184.216.34\n")
        .create();

    let mut dynu_server = mockito::Server::new();
    let _m_dynu = dynu_server
        .mock("GET", "/nic/update")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body("good 93.184.216.34\n")
        .create();

    let temp_dir = std::env::temp_dir().join("ddns_e2e_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let state_path = temp_dir.join("state.json");
    let history_path = temp_dir.join("history.jsonl");

    let toml_content = format!(
        r#"
[ip_providers]
urls = ["{}/ip"]
timeout_secs = 2

[[subscribers]]
provider = "dynu"
domain = "mytest.dyndns.org"
credentials_env = "E2E_DYNU_SECRET"
"#,
        ip_server.url()
    );

    unsafe {
        std::env::set_var("E2E_DYNU_SECRET", "super_secret");
    }

    let config = AppConfig::from_toml(&toml_content).unwrap();

    let resolver = HttpIpResolver::new(
        config.ip_providers.urls.clone(),
        Duration::from_secs(config.ip_providers.timeout_secs),
    );
    let state_repo = JsonStateRepository::new(&state_path);
    let history_repo = JsonlHistoryRepository::new(&history_path);

    let mut updaters: Vec<Box<dyn DdnsUpdater>> = Vec::new();
    for sub in &config.subscribers {
        let cred = sub.resolve_credential().unwrap();
        let dynu_endpoint = format!("{}/nic/update", dynu_server.url());
        let updater = DynuUpdater::with_endpoint(
            &sub.domain,
            &cred,
            dynu_endpoint,
            Duration::from_secs(config.ip_providers.timeout_secs),
        );
        updaters.push(Box::new(updater));
    }

    let service = IpMonitorService::new(
        Box::new(resolver),
        Box::new(state_repo),
        Box::new(history_repo),
        updaters,
    );

    // First run: detects IP, updates Dynu, saves state
    let outcome = service.run().expect("first run succeeds");
    match outcome {
        RunOutcome::Updated { ip, subscribers } => {
            assert_eq!(ip, Ipv4Addr::new(93, 184, 216, 34));
            assert_eq!(subscribers.len(), 1);
            assert!(subscribers[0].success);
        }
        other => panic!("Expected Updated, got {:?}", other),
    }

    // Verify files created on disk
    assert!(state_path.exists());
    assert!(history_path.exists());

    // Second run: IP hasn't changed -> NoChange outcome
    let second_outcome = service.run().expect("second run succeeds");
    match second_outcome {
        RunOutcome::NoChange { ip } => {
            assert_eq!(ip, Ipv4Addr::new(93, 184, 216, 34));
        }
        other => panic!("Expected NoChange, got {:?}", other),
    }

    unsafe {
        std::env::remove_var("E2E_DYNU_SECRET");
    }
    let _ = fs::remove_dir_all(&temp_dir);
}
