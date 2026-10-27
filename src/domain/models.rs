use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Persisted state of the monitor across executions.
///
/// `last_ip` reflects the IP address from the last *fully successful* fan-out cycle.
/// It must never be updated if any subscriber fails.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MonitorState {
    /// Index of the IP-check provider to query on the next run.
    pub last_provider_index: Option<usize>,
    /// Public IP address from the last successfully completed DDNS update.
    pub last_ip: Option<Ipv4Addr>,
}

/// Result of a successful public IP discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpDiscoveryResult {
    /// The discovered public IPv4 address.
    pub ip: Ipv4Addr,
    /// Display name or URL of the provider that responded.
    pub provider: String,
    /// Index of the provider in the configured list (for rotation persistence).
    pub provider_index: usize,
}

/// Result of notifying an individual DDNS subscriber.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriberResult {
    /// The name of the DDNS provider (e.g. "dynu").
    pub provider: String,
    /// Whether the update was accepted by the provider.
    pub success: bool,
    /// Optional error details if the update failed.
    pub error: Option<String>,
}

/// A persistent audit record appended to the JSONL history log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// ISO 8601 UTC timestamp.
    pub timestamp: String,
    /// Public IP discovered for this run.
    pub ip: Ipv4Addr,
    /// Overall run status ("success", "partial", "failed", "no_change").
    pub status: String,
    /// Results for each individual subscriber notified.
    pub subscribers: Vec<SubscriberResult>,
}
