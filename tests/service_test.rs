use ddns_monitor::domain::errors::{
    HistoryError, ResolveError, StateError, UpdateError,
};
use ddns_monitor::domain::models::{HistoryEntry, IpDiscoveryResult, MonitorState};
use ddns_monitor::domain::traits::{DdnsUpdater, HistoryRepository, IpResolver, StateRepository};
use ddns_monitor::service::{IpMonitorService, RunOutcome};
use std::net::Ipv4Addr;
use std::sync::Mutex;

// --- Test Fakes ---

struct FakeResolver {
    result: Result<IpDiscoveryResult, String>,
}

impl IpResolver for FakeResolver {
    fn resolve(&self, _last_index: Option<usize>) -> Result<IpDiscoveryResult, ResolveError> {
        self.result
            .clone()
            .map_err(ResolveError::AllProvidersFailed)
    }
}

#[derive(Default)]
struct FakeStateRepo {
    state: Mutex<MonitorState>,
}

impl StateRepository for FakeStateRepo {
    fn load(&self) -> Result<MonitorState, StateError> {
        Ok(self.state.lock().unwrap().clone())
    }

    fn save(&self, state: &MonitorState) -> Result<(), StateError> {
        *self.state.lock().unwrap() = state.clone();
        Ok(())
    }
}

#[derive(Default)]
struct FakeHistoryRepo {
    entries: Mutex<Vec<HistoryEntry>>,
}

impl HistoryRepository for FakeHistoryRepo {
    fn append(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }
}

struct FakeUpdater {
    name: String,
    succeed: bool,
    calls: Mutex<Vec<Ipv4Addr>>,
}

impl FakeUpdater {
    fn new(name: &str, succeed: bool) -> Self {
        Self {
            name: name.to_string(),
            succeed,
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl DdnsUpdater for FakeUpdater {
    fn name(&self) -> &str {
        &self.name
    }

    fn update(&self, ip: Ipv4Addr) -> Result<(), UpdateError> {
        self.calls.lock().unwrap().push(ip);
        if self.succeed {
            Ok(())
        } else {
            Err(UpdateError::ProviderRejected {
                provider: self.name.clone(),
                message: "Simulated rejection".to_string(),
            })
        }
    }
}

// --- Tests ---

#[test]
fn test_no_change_when_ip_matches() {
    let old_ip = Ipv4Addr::new(93, 184, 216, 34);
    let state_repo = FakeStateRepo::default();
    state_repo
        .save(&MonitorState {
            last_provider_index: Some(0),
            last_ip: Some(old_ip),
        })
        .unwrap();

    let history_repo = FakeHistoryRepo::default();
    let resolver = FakeResolver {
        result: Ok(IpDiscoveryResult {
            ip: old_ip,
            provider: "fake_provider".to_string(),
            provider_index: 1,
        }),
    };

    let updater = FakeUpdater::new("dynu", true);

    let service = IpMonitorService::new(
        Box::new(resolver),
        Box::new(state_repo),
        Box::new(history_repo),
        vec![Box::new(updater)],
    );

    let outcome = service.run().expect("service run succeeds");
    match outcome {
        RunOutcome::NoChange { ip } => assert_eq!(ip, old_ip),
        other => panic!("Expected NoChange, got: {:?}", other),
    }

    // Verify last_ip unchanged, but last_provider_index rotated to 1
    let saved_state = service.state_repo.load().unwrap();
    assert_eq!(saved_state.last_ip, Some(old_ip));
    assert_eq!(saved_state.last_provider_index, Some(1));
}

#[test]
fn test_successful_ip_change_updates_last_ip() {
    let old_ip = Ipv4Addr::new(93, 184, 216, 34);
    let new_ip = Ipv4Addr::new(198, 51, 100, 1);

    let state_repo = FakeStateRepo::default();
    state_repo
        .save(&MonitorState {
            last_provider_index: Some(0),
            last_ip: Some(old_ip),
        })
        .unwrap();

    let history_repo = FakeHistoryRepo::default();
    let resolver = FakeResolver {
        result: Ok(IpDiscoveryResult {
            ip: new_ip,
            provider: "fake_provider".to_string(),
            provider_index: 1,
        }),
    };

    let updater1 = FakeUpdater::new("dynu1", true);
    let updater2 = FakeUpdater::new("dynu2", true);

    let service = IpMonitorService::new(
        Box::new(resolver),
        Box::new(state_repo),
        Box::new(history_repo),
        vec![Box::new(updater1), Box::new(updater2)],
    );

    let outcome = service.run().expect("service run succeeds");
    match outcome {
        RunOutcome::Updated { ip, subscribers } => {
            assert_eq!(ip, new_ip);
            assert_eq!(subscribers.len(), 2);
            assert!(subscribers.iter().all(|s| s.success));
        }
        other => panic!("Expected Updated, got: {:?}", other),
    }

    // Verify last_ip updated to new_ip
    let saved_state = service.state_repo.load().unwrap();
    assert_eq!(saved_state.last_ip, Some(new_ip));
    assert_eq!(saved_state.last_provider_index, Some(1));
}

#[test]
fn test_partial_failure_preserves_old_last_ip() {
    let old_ip = Ipv4Addr::new(93, 184, 216, 34);
    let new_ip = Ipv4Addr::new(198, 51, 100, 1);

    let state_repo = FakeStateRepo::default();
    state_repo
        .save(&MonitorState {
            last_provider_index: Some(0),
            last_ip: Some(old_ip),
        })
        .unwrap();

    let history_repo = FakeHistoryRepo::default();
    let resolver = FakeResolver {
        result: Ok(IpDiscoveryResult {
            ip: new_ip,
            provider: "fake_provider".to_string(),
            provider_index: 1,
        }),
    };

    let updater1 = FakeUpdater::new("dynu1", true);
    let updater2 = FakeUpdater::new("dynu2", false); // fails!

    let service = IpMonitorService::new(
        Box::new(resolver),
        Box::new(state_repo),
        Box::new(history_repo),
        vec![Box::new(updater1), Box::new(updater2)],
    );

    let outcome = service.run().expect("service run returns partial status");
    match outcome {
        RunOutcome::PartialFailure { ip, subscribers } => {
            assert_eq!(ip, new_ip);
            assert_eq!(subscribers.len(), 2);
            assert!(subscribers.iter().any(|s| !s.success));
        }
        other => panic!("Expected PartialFailure, got: {:?}", other),
    }

    // CRITICAL INVARIANT: last_ip MUST NOT advance on partial failure!
    let saved_state = service.state_repo.load().unwrap();
    assert_eq!(saved_state.last_ip, Some(old_ip));
    // But last_provider_index is updated to preserve rotation
    assert_eq!(saved_state.last_provider_index, Some(1));
}

#[test]
fn test_all_failed_preserves_old_last_ip() {
    let old_ip = Ipv4Addr::new(93, 184, 216, 34);
    let new_ip = Ipv4Addr::new(198, 51, 100, 1);

    let state_repo = FakeStateRepo::default();
    state_repo
        .save(&MonitorState {
            last_provider_index: Some(0),
            last_ip: Some(old_ip),
        })
        .unwrap();

    let history_repo = FakeHistoryRepo::default();
    let resolver = FakeResolver {
        result: Ok(IpDiscoveryResult {
            ip: new_ip,
            provider: "fake_provider".to_string(),
            provider_index: 2,
        }),
    };

    let updater1 = FakeUpdater::new("dynu1", false);
    let updater2 = FakeUpdater::new("dynu2", false);

    let service = IpMonitorService::new(
        Box::new(resolver),
        Box::new(state_repo),
        Box::new(history_repo),
        vec![Box::new(updater1), Box::new(updater2)],
    );

    let outcome = service.run().expect("service run returns all failed status");
    match outcome {
        RunOutcome::AllFailed { ip, subscribers } => {
            assert_eq!(ip, new_ip);
            assert_eq!(subscribers.len(), 2);
            assert!(subscribers.iter().all(|s| !s.success));
        }
        other => panic!("Expected AllFailed, got: {:?}", other),
    }

    let saved_state = service.state_repo.load().unwrap();
    assert_eq!(saved_state.last_ip, Some(old_ip));
    assert_eq!(saved_state.last_provider_index, Some(2));
}
