use std::net::Ipv4Addr;

use crate::domain::errors::{HistoryError, ResolveError, StateError, UpdateError};
use crate::domain::models::{HistoryEntry, IpDiscoveryResult, MonitorState};

/// Contract for resolving the current public WAN IPv4 address.
pub trait IpResolver: Send + Sync {
    /// Discovers the current public IPv4 address, starting from or prioritizing
    /// rotation based on `last_index`.
    fn resolve(&self, last_index: Option<usize>) -> Result<IpDiscoveryResult, ResolveError>;
}

/// Contract for updating a dynamic DNS provider with a new IP.
pub trait DdnsUpdater: Send + Sync {
    /// Human-readable identifier for the updater (e.g. "dynu:example.com").
    fn name(&self) -> &str;

    /// Dispatches the update to the DNS provider.
    fn update(&self, ip: Ipv4Addr) -> Result<(), UpdateError>;
}

/// Contract for persisting and loading the monitor state across runs.
pub trait StateRepository: Send + Sync {
    /// Loads the stored monitor state, returning `MonitorState::default()` if absent.
    fn load(&self) -> Result<MonitorState, StateError>;

    /// Atomically persists the updated monitor state.
    fn save(&self, state: &MonitorState) -> Result<(), StateError>;
}

/// Contract for appending execution records to the audit history log.
pub trait HistoryRepository: Send + Sync {
    /// Appends an entry to the log.
    fn append(&self, entry: &HistoryEntry) -> Result<(), HistoryError>;
}
