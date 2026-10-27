use ddns_monitor::domain::models::{HistoryEntry, MonitorState, SubscriberResult};
use ddns_monitor::domain::traits::{HistoryRepository, StateRepository};
use ddns_monitor::infra::history_store::JsonlHistoryRepository;
use ddns_monitor::infra::state_store::JsonStateRepository;
use std::fs;
use std::net::Ipv4Addr;

#[test]
fn test_state_repo_load_nonexistent_returns_default() {
    let tmp_dir = std::env::temp_dir().join("ddns_test_nonexistent");
    let state_file = tmp_dir.join("state.json");
    let _ = fs::remove_file(&state_file);

    let repo = JsonStateRepository::new(state_file);
    let state = repo.load().expect("load default state");

    assert_eq!(state, MonitorState::default());
    assert_eq!(state.last_ip, None);
    assert_eq!(state.last_provider_index, None);
}

#[test]
fn test_state_repo_atomic_save_and_load() {
    let tmp_dir = std::env::temp_dir().join("ddns_test_save_load");
    let state_file = tmp_dir.join("state.json");
    let _ = fs::remove_dir_all(&tmp_dir);

    let repo = JsonStateRepository::new(state_file.clone());

    let state = MonitorState {
        last_provider_index: Some(2),
        last_ip: Some(Ipv4Addr::new(93, 184, 216, 34)),
    };

    repo.save(&state).expect("save state");
    assert!(state_file.exists());

    let loaded = repo.load().expect("load state");
    assert_eq!(state, loaded);

    // Save update
    let updated = MonitorState {
        last_provider_index: Some(0),
        last_ip: Some(Ipv4Addr::new(198, 51, 100, 1)),
    };
    repo.save(&updated).expect("update state");
    let loaded_updated = repo.load().expect("load updated state");
    assert_eq!(updated, loaded_updated);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_history_repo_append_and_verify() {
    let tmp_dir = std::env::temp_dir().join("ddns_test_history");
    let history_file = tmp_dir.join("history.jsonl");
    let _ = fs::remove_dir_all(&tmp_dir);

    let repo = JsonlHistoryRepository::new(history_file.clone());

    let entry1 = HistoryEntry {
        timestamp: "2026-10-27T12:00:00Z".to_string(),
        ip: Ipv4Addr::new(93, 184, 216, 34),
        status: "success".to_string(),
        subscribers: vec![SubscriberResult {
            provider: "dynu".to_string(),
            success: true,
            error: None,
        }],
    };

    let entry2 = HistoryEntry {
        timestamp: "2026-10-27T12:30:00Z".to_string(),
        ip: Ipv4Addr::new(198, 51, 100, 1),
        status: "partial".to_string(),
        subscribers: vec![SubscriberResult {
            provider: "dynu".to_string(),
            success: false,
            error: Some("Timeout".to_string()),
        }],
    };

    repo.append(&entry1).expect("append entry 1");
    repo.append(&entry2).expect("append entry 2");

    let content = fs::read_to_string(&history_file).expect("read history file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    let parsed1: HistoryEntry = serde_json::from_str(lines[0]).expect("parse line 1");
    let parsed2: HistoryEntry = serde_json::from_str(lines[1]).expect("parse line 2");

    assert_eq!(parsed1, entry1);
    assert_eq!(parsed2, entry2);

    let _ = fs::remove_dir_all(&tmp_dir);
}
