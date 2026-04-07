//! Configuration types for building a [`crate::Gateway`].

use std::time::Duration;

use crate::protocol::ProviderEndpoint;

/// Configuration for a single API key.
#[derive(Debug, Clone)]
pub struct KeyConfig {
    /// The raw API key string sent according to the provider auth scheme.
    pub(crate) key: String,
    /// Human-readable label for observability (e.g. `"openai-prod-1"`).
    pub(crate) label: String,
    /// Hard TPM cap for this key.
    pub(crate) tpm_limit: u32,
    /// RPM limit for this key's sliding window.
    pub(crate) rpm_limit: u32,
}

impl KeyConfig {
    /// Create a new key configuration with sensible defaults.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            tpm_limit: 90_000,
            rpm_limit: 500,
        }
    }

    pub fn tpm_limit(mut self, limit: u32) -> Self {
        self.tpm_limit = limit;
        self
    }

    pub fn rpm_limit(mut self, limit: u32) -> Self {
        self.rpm_limit = limit;
        self
    }
}

/// Tuning parameters for the key pool's acquire and error-handling behaviour.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_cas_attempts: usize,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_cooldown: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_cas_attempts: 5,
            circuit_breaker_threshold: 5,
            circuit_breaker_cooldown: Duration::from_secs(30),
        }
    }
}

/// Top-level configuration for constructing a [`crate::Gateway`].
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub(crate) provider_endpoint: ProviderEndpoint,
    pub(crate) keys: Vec<KeyConfig>,
    pub(crate) budget_limit_usd: Option<f64>,
    pub(crate) pool_config: PoolConfig,
    pub(crate) request_timeout: Duration,
}

impl GatewayConfig {
    pub fn provider_endpoint(&self) -> &ProviderEndpoint {
        &self.provider_endpoint
    }

    pub fn budget_limit_usd(&self) -> Option<f64> {
        self.budget_limit_usd
    }
}
