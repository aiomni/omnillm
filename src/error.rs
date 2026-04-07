//! Error types for the omnillm crate.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use thiserror::Error;

use crate::protocol::ProviderProtocol;

/// Arbitrary provider-specific error payload.
pub type ErrorExtensions = BTreeMap<String, Value>;

/// A normalized provider error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderError {
    pub protocol: ProviderProtocol,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: ErrorExtensions,
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} error", self.protocol)?;
        if let Some(status) = self.status {
            write!(f, " ({status})")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for ProviderError {}

/// Errors returned to the caller of [`crate::Gateway::call`].
#[derive(Debug, Error)]
pub enum GatewayError {
    /// No healthy key with sufficient TPM capacity is available.
    #[error("no healthy key with sufficient TPM capacity")]
    NoAvailableKey,

    /// The budget limit has been exceeded.
    #[error("budget limit exceeded")]
    BudgetExceeded,

    /// The request was rate-limited by the local RPM sliding window.
    #[error("rate limited by local RPM window")]
    RateLimited,

    /// The provider returned HTTP 401/403 — the key is permanently dead.
    #[error("provider returned 401/403 — key is dead")]
    Unauthorized,

    /// The request was cancelled by the upstream caller.
    #[error("request cancelled by upstream")]
    Cancelled,

    /// Provider-side failure.
    #[error(transparent)]
    Provider(ProviderError),

    /// Local canonical/protocol conversion failed before or after transport.
    #[error("protocol conversion error: {0}")]
    Protocol(String),

    /// An HTTP transport error from the underlying client.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Internal error classification used by [`crate::key::pool::KeyPool`] to
/// drive key state transitions.
#[derive(Debug)]
pub(crate) enum ApiError {
    /// HTTP 401/403 — key is permanently invalid.
    Unauthorized,
    /// HTTP 429 — provider asks us to back off.
    RateLimited { retry_after: Duration },
    /// Provider-side failure.
    Provider(ProviderError),
    /// Local protocol conversion failure.
    Protocol(String),
    /// Upstream caller cancelled the request.
    Cancelled,
}
