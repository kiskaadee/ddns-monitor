use std::net::Ipv4Addr;

/// Persisted state of the monitor.
///
/// Loaded at startup and saved after each successful update cycle.
/// `last_ip` is the IP address from the last *fully successful* fan-out —
/// it is not updated merely because a new IP was discovered.
#[derive(Debug, Clone, Default)]
pub struct MonitorState {
    /// Index of the IP-check provider to query on the next run.
    pub last_provider_index: Option<usize>,
    /// Public IP address from the last successfully completed DDNS update.
    pub last_ip: Option<Ipv4Addr>,
}

/// Result of a successful public IP discovery.
#[derive(Debug, Clone)]
pub struct IpDiscoveryResult {
    /// The discovered public IPv4 address.
    pub ip: Ipv4Addr,
    /// Display name of the provider that responded.
    pub provider: String,
    /// Index of the provider in the configured list (for rotation persistence).
    pub provider_index: usize,
}
