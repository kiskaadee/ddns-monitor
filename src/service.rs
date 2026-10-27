use std::net::Ipv4Addr;
use thiserror::Error;

use crate::domain::errors::{HistoryError, ResolveError, StateError};
use crate::domain::models::{HistoryEntry, SubscriberResult};
use crate::domain::traits::{DdnsUpdater, HistoryRepository, IpResolver, StateRepository};

/// High-level errors encountered by the monitor orchestrator.
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("WAN IP resolution failed: {0}")]
    Resolve(#[from] ResolveError),

    #[error("State repository error: {0}")]
    State(#[from] StateError),

    #[error("History audit logging error: {0}")]
    History(#[from] HistoryError),
}

/// The outcome of an execution cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// WAN IP has not changed since the last fully successful update.
    NoChange { ip: Ipv4Addr },

    /// WAN IP changed and all DDNS subscribers were successfully updated.
    Updated {
        ip: Ipv4Addr,
        subscribers: Vec<SubscriberResult>,
    },

    /// WAN IP changed, but only a subset of subscribers succeeded.
    /// `last_ip` was NOT updated.
    PartialFailure {
        ip: Ipv4Addr,
        subscribers: Vec<SubscriberResult>,
    },

    /// WAN IP changed, but all subscribers failed.
    /// `last_ip` was NOT updated.
    AllFailed {
        ip: Ipv4Addr,
        subscribers: Vec<SubscriberResult>,
    },
}

/// Core orchestrator for WAN IP change detection and DDNS dispatching.
pub struct IpMonitorService {
    pub resolver: Box<dyn IpResolver>,
    pub state_repo: Box<dyn StateRepository>,
    pub history_repo: Box<dyn HistoryRepository>,
    pub updaters: Vec<Box<dyn DdnsUpdater>>,
}

impl IpMonitorService {
    pub fn new(
        resolver: Box<dyn IpResolver>,
        state_repo: Box<dyn StateRepository>,
        history_repo: Box<dyn HistoryRepository>,
        updaters: Vec<Box<dyn DdnsUpdater>>,
    ) -> Self {
        Self {
            resolver,
            state_repo,
            history_repo,
            updaters,
        }
    }

    /// Executes one cycle of IP discovery, comparison, and fan-out dispatching.
    pub fn run(&self) -> Result<RunOutcome, ServiceError> {
        let mut state = self.state_repo.load()?;
        log::debug!("Loaded existing state: {:?}", state);

        let discovery = self.resolver.resolve(state.last_provider_index)?;
        log::info!(
            "Discovered public IP {} via provider '{}' (index {})",
            discovery.ip,
            discovery.provider,
            discovery.provider_index
        );

        if state.last_ip == Some(discovery.ip) {
            log::info!("IP matches last successful IP ({}). No update needed.", discovery.ip);
            // Persist the updated provider index so rotation continues
            state.last_provider_index = Some(discovery.provider_index);
            self.state_repo.save(&state)?;
            return Ok(RunOutcome::NoChange { ip: discovery.ip });
        }

        log::info!(
            "IP change detected! Old: {:?}, New: {}. Fan-out to {} subscriber(s)",
            state.last_ip,
            discovery.ip,
            self.updaters.len()
        );

        // Fan out to all subscribers
        let mut subscriber_results = Vec::new();
        let mut success_count = 0;

        for updater in &self.updaters {
            let name = updater.name().to_string();
            match updater.update(discovery.ip) {
                Ok(()) => {
                    log::info!("Subscriber '{}' update succeeded.", name);
                    subscriber_results.push(SubscriberResult {
                        provider: name,
                        success: true,
                        error: None,
                    });
                    success_count += 1;
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    log::error!("Subscriber '{}' update failed: {}", name, err_msg);
                    subscriber_results.push(SubscriberResult {
                        provider: name,
                        success: false,
                        error: Some(err_msg),
                    });
                }
            }
        }

        let total = self.updaters.len();
        let (status_str, outcome) = if total == 0 || success_count == total {
            (
                "success",
                RunOutcome::Updated {
                    ip: discovery.ip,
                    subscribers: subscriber_results.clone(),
                },
            )
        } else if success_count > 0 {
            (
                "partial",
                RunOutcome::PartialFailure {
                    ip: discovery.ip,
                    subscribers: subscriber_results.clone(),
                },
            )
        } else {
            (
                "failed",
                RunOutcome::AllFailed {
                    ip: discovery.ip,
                    subscribers: subscriber_results.clone(),
                },
            )
        };

        // Record entry in audit history
        let timestamp = jiff::Timestamp::now().to_string();
        let entry = HistoryEntry {
            timestamp,
            ip: discovery.ip,
            status: status_str.to_string(),
            subscribers: subscriber_results,
        };
        if let Err(e) = self.history_repo.append(&entry) {
            log::warn!("Failed to append to history repository: {}", e);
        }

        // Apply state advancement invariants
        state.last_provider_index = Some(discovery.provider_index);
        if status_str == "success" {
            state.last_ip = Some(discovery.ip);
        }
        self.state_repo.save(&state)?;

        Ok(outcome)
    }
}
