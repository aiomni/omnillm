//! Provider → Model → KeyPool routing.

use std::collections::HashMap;
use std::sync::Arc;

use super::inner::KeyInner;
use super::lease::KeyLease;
use super::pool::KeyPool;
use crate::config::{KeyConfig, PoolConfig};

/// A string alias for provider identifiers (e.g. `"openai"`, `"anthropic"`).
pub type ProviderId = String;

/// A string alias for model identifiers (e.g. `"gpt-4o"`, `"claude-3-5-sonnet"`).
pub type ModelId = String;

/// Routes requests to the correct [`KeyPool`] based on provider and model.
///
/// Keys are not a flat list. Different models under the same provider have
/// independent rate limits — a GPT-4o key's TPM quota is separate from its
/// GPT-4o-mini quota. `PoolRegistry` enforces this hierarchy.
pub struct PoolRegistry {
    pools: HashMap<(ProviderId, ModelId), KeyPool>,
}

impl PoolRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    /// Register a pool of keys for a specific provider/model combination.
    pub fn register(
        &mut self,
        provider: ProviderId,
        model: ModelId,
        keys: Vec<KeyConfig>,
        config: PoolConfig,
    ) {
        let inner_keys: Vec<Arc<KeyInner>> = keys
            .into_iter()
            .map(|kc| Arc::new(KeyInner::new(kc.key, kc.label, kc.tpm_limit, kc.rpm_limit)))
            .collect();
        self.pools
            .insert((provider, model), KeyPool::new(inner_keys, config));
    }

    /// Acquire a key from the pool for the given provider and model.
    pub fn acquire(&self, provider: &str, model: &str, estimated_tokens: u32) -> Option<KeyLease> {
        self.pools
            .get(&(provider.to_string(), model.to_string()))?
            .acquire(estimated_tokens)
    }
}

impl Default for PoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
