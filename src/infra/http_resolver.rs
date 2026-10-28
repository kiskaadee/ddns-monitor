use std::net::Ipv4Addr;
use std::time::Duration;
use ureq::Agent;

use crate::domain::errors::ResolveError;
use crate::domain::models::IpDiscoveryResult;
use crate::domain::traits::IpResolver;
use crate::domain::validation::{is_valid_public_ipv4, next_provider_index};

/// Resolves the public WAN IPv4 address by querying a sequence of HTTP echo services.
pub struct HttpIpResolver {
    urls: Vec<String>,
    agent: Agent,
}

impl HttpIpResolver {
    pub fn new(urls: Vec<String>, timeout: Duration) -> Self {
        let config = ureq::config::Config::builder()
            .timeout_global(Some(timeout))
            .build();
        let agent = Agent::new_with_config(config);
        Self { urls, agent }
    }
}

impl IpResolver for HttpIpResolver {
    fn resolve(&self, last_index: Option<usize>) -> Result<IpDiscoveryResult, ResolveError> {
        let n = self.urls.len();
        if n == 0 {
            return Err(ResolveError::NoProvidersConfigured);
        }

        let start_index = next_provider_index(last_index, n).unwrap_or(0);
        let mut errors = Vec::new();

        for offset in 0..n {
            let current_index = (start_index + offset) % n;
            let url = &self.urls[current_index];

            log::debug!(
                "Attempting IP discovery with provider '{}' (index {})",
                url,
                current_index
            );

            let resp = self
                .agent
                .get(url)
                .header("User-Agent", "ddns-monitor/0.1.0")
                .call();

            let mut response = match resp {
                Ok(r) => r,
                Err(e) => {
                    let err_msg = format!("{}: HTTP error: {}", url, e);
                    log::warn!("{}", err_msg);
                    errors.push(err_msg);
                    continue;
                }
            };

            let body_str = match response.body_mut().read_to_string() {
                Ok(s) => s,
                Err(e) => {
                    let err_msg = format!("{}: failed reading response body: {}", url, e);
                    log::warn!("{}", err_msg);
                    errors.push(err_msg);
                    continue;
                }
            };

            let trimmed = body_str.trim();
            match trimmed.parse::<Ipv4Addr>() {
                Ok(ip) => {
                    if is_valid_public_ipv4(ip) {
                        return Ok(IpDiscoveryResult {
                            ip,
                            provider: url.clone(),
                            provider_index: current_index,
                        });
                    } else {
                        let err_msg = format!("{}: received non-public IP '{}'", url, trimmed);
                        log::warn!("{}", err_msg);
                        errors.push(err_msg);
                    }
                }
                Err(e) => {
                    let err_msg = format!("{}: invalid IPv4 string '{}': {}", url, trimmed, e);
                    log::warn!("{}", err_msg);
                    errors.push(err_msg);
                }
            }
        }

        Err(ResolveError::AllProvidersFailed(errors.join("; ")))
    }
}
