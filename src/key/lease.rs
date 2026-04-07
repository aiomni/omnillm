//! RAII quota guard for TPM reservations.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::inner::KeyInner;

/// An RAII guard that holds a TPM reservation against a specific key.
///
/// When this lease is dropped — whether the call succeeded, failed, panicked,
/// or was cancelled — the reserved tokens are unconditionally returned via
/// `fetch_sub`. There is no code path where quota can be leaked.
pub struct KeyLease {
    pub(crate) inner: Arc<KeyInner>,
    pub(crate) reserved_tokens: u32,
}

impl KeyLease {
    /// Returns the human-readable label of the leased key.
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    /// Returns the number of tokens reserved by this lease.
    pub fn reserved_tokens(&self) -> u32 {
        self.reserved_tokens
    }
}

impl Drop for KeyLease {
    fn drop(&mut self) {
        self.inner
            .tpm_inflight
            .fetch_sub(self.reserved_tokens, Ordering::Release);
    }
}
