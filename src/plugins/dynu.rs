use std::net::Ipv4Addr;
use std::time::Duration;
use ureq::Agent;

use crate::domain::errors::UpdateError;
use crate::domain::traits::DdnsUpdater;

pub const DEFAULT_DYNU_ENDPOINT: &str = "https://api.dynu.com/nic/update";

/// DDNS Updater plugin for Dynu Systems.
///
/// Communicates directly with the Dynu IP update protocol, eliminating
/// the external `ddclient` daemon dependency.
pub struct DynuUpdater {
    domain: String,
    credential: String,
    endpoint: String,
    agent: Agent,
}

impl DynuUpdater {
    pub fn new(domain: &str, credential: &str, timeout: Duration) -> Self {
        Self::with_endpoint(domain, credential, DEFAULT_DYNU_ENDPOINT, timeout)
    }

    pub fn with_endpoint<S: Into<String>>(
        domain: &str,
        credential: &str,
        endpoint: S,
        timeout: Duration,
    ) -> Self {
        let config = ureq::config::Config::builder()
            .timeout_global(Some(timeout))
            .build();
        let agent = Agent::new_with_config(config);
        Self {
            domain: domain.to_string(),
            credential: credential.to_string(),
            endpoint: endpoint.into(),
            agent,
        }
    }
}

impl DdnsUpdater for DynuUpdater {
    fn name(&self) -> &str {
        &self.domain
    }

    fn update(&self, ip: Ipv4Addr) -> Result<(), UpdateError> {
        log::info!("Updating Dynu record for domain '{}' to {}", self.domain, ip);

        let resp = self
            .agent
            .get(&self.endpoint)
            .query("hostname", &self.domain)
            .query("myip", ip.to_string())
            .query("password", &self.credential)
            .header("User-Agent", "ddns-monitor/0.1.0")
            .call();

        let mut response = resp.map_err(|e| UpdateError::NetworkError {
            provider: format!("dynu:{}", self.domain),
            details: e.to_string(),
        })?;

        let status_code = response.status();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|e| UpdateError::NetworkError {
                provider: format!("dynu:{}", self.domain),
                details: format!("Failed reading response: {}", e),
            })?;

        let trimmed = body.trim();
        log::debug!(
            "Dynu response (status {}): '{}'",
            status_code,
            trimmed
        );

        if !status_code.is_success() {
            return Err(UpdateError::NetworkError {
                provider: format!("dynu:{}", self.domain),
                details: format!("HTTP {} from Dynu: {}", status_code, trimmed),
            });
        }

        if trimmed.starts_with("good") || trimmed.starts_with("nochg") {
            log::info!("Dynu update confirmed for '{}': {}", self.domain, trimmed);
            Ok(())
        } else if trimmed.starts_with("badauth") {
            Err(UpdateError::AuthError {
                provider: format!("dynu:{}", self.domain),
                details: "Invalid Dynu credentials or password".to_string(),
            })
        } else {
            Err(UpdateError::ProviderRejected {
                provider: format!("dynu:{}", self.domain),
                message: trimmed.to_string(),
            })
        }
    }
}
