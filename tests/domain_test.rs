use std::net::Ipv4Addr;
use ddns_monitor::domain::models::{HistoryEntry, IpDiscoveryResult, MonitorState, SubscriberResult};
use ddns_monitor::domain::validation::{is_valid_public_ipv4, next_provider_index};

#[test]
fn test_is_valid_public_ipv4() {
    // Valid public IPv4 addresses
    assert!(is_valid_public_ipv4("8.8.8.8".parse().unwrap()));
    assert!(is_valid_public_ipv4("1.1.1.1".parse().unwrap()));
    assert!(is_valid_public_ipv4("93.184.216.34".parse().unwrap()));

    // RFC 1918 Private ranges
    assert!(!is_valid_public_ipv4("10.0.0.1".parse().unwrap()));
    assert!(!is_valid_public_ipv4("10.255.255.255".parse().unwrap()));
    assert!(!is_valid_public_ipv4("172.16.0.1".parse().unwrap()));
    assert!(!is_valid_public_ipv4("172.31.255.255".parse().unwrap()));
    assert!(!is_valid_public_ipv4("192.168.0.1".parse().unwrap()));
    assert!(!is_valid_public_ipv4("192.168.255.255".parse().unwrap()));

    // RFC 6598 Carrier-Grade NAT (100.64.0.0/10)
    assert!(!is_valid_public_ipv4("100.64.0.1".parse().unwrap()));
    assert!(!is_valid_public_ipv4("100.127.255.254".parse().unwrap()));

    // Loopback (127.0.0.0/8)
    assert!(!is_valid_public_ipv4("127.0.0.1".parse().unwrap()));
    assert!(!is_valid_public_ipv4("127.255.255.255".parse().unwrap()));

    // Link-local (169.254.0.0/16)
    assert!(!is_valid_public_ipv4("169.254.0.1".parse().unwrap()));
    assert!(!is_valid_public_ipv4("169.254.255.255".parse().unwrap()));

    // Broadcast, unspecified, multicast
    assert!(!is_valid_public_ipv4("0.0.0.0".parse().unwrap()));
    assert!(!is_valid_public_ipv4("255.255.255.255".parse().unwrap()));
    assert!(!is_valid_public_ipv4("224.0.0.1".parse().unwrap()));
}

#[test]
fn test_next_provider_index() {
    assert_eq!(next_provider_index(None, 3), Some(0));
    assert_eq!(next_provider_index(Some(0), 3), Some(1));
    assert_eq!(next_provider_index(Some(1), 3), Some(2));
    assert_eq!(next_provider_index(Some(2), 3), Some(0)); // wraps around
    assert_eq!(next_provider_index(None, 0), None);
    assert_eq!(next_provider_index(Some(0), 0), None);
}

#[test]
fn test_monitor_state_serialization() {
    let state = MonitorState {
        last_provider_index: Some(2),
        last_ip: Some(Ipv4Addr::new(93, 184, 216, 34)),
    };

    let serialized = serde_json::to_string(&state).expect("serialize monitor state");
    let deserialized: MonitorState =
        serde_json::from_str(&serialized).expect("deserialize monitor state");

    assert_eq!(state, deserialized);
}

#[test]
fn test_history_entry_serialization() {
    let discovery = IpDiscoveryResult {
        ip: Ipv4Addr::new(93, 184, 216, 34),
        provider: "https://api.ipify.org".to_string(),
        provider_index: 0,
    };
    let disc_serialized = serde_json::to_string(&discovery).expect("serialize discovery");
    let disc_deserialized: IpDiscoveryResult =
        serde_json::from_str(&disc_serialized).expect("deserialize discovery");
    assert_eq!(discovery, disc_deserialized);

    let entry = HistoryEntry {
        timestamp: "2026-10-27T12:00:00Z".to_string(),
        ip: Ipv4Addr::new(93, 184, 216, 34),
        status: "success".to_string(),
        subscribers: vec![SubscriberResult {
            provider: "dynu".to_string(),
            success: true,
            error: None,
        }],
    };

    let serialized = serde_json::to_string(&entry).expect("serialize history entry");
    let deserialized: HistoryEntry =
        serde_json::from_str(&serialized).expect("deserialize history entry");

    assert_eq!(entry, deserialized);
}
