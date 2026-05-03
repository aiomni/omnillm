use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VendorExtensions;

/// Unified capability layer for model generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<StructuredOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modalities: Vec<OutputModality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety: Option<SafetySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache: Option<PromptCachePolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub builtin_tools: Vec<BuiltinTool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl CapabilitySet {
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
            && self.structured_output.is_none()
            && self.reasoning.is_none()
            && self.modalities.is_empty()
            && self.safety.is_none()
            && self.cache.is_none()
            && self.prompt_cache.is_none()
            && self.builtin_tools.is_empty()
            && self.vendor_extensions.is_empty()
    }

    pub fn effective_prompt_cache(&self) -> Option<PromptCachePolicy> {
        self.prompt_cache.clone().or_else(|| {
            self.cache
                .clone()
                .map(PromptCachePolicy::from_legacy_cache_settings)
        })
    }
}

/// A callable custom tool/function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strict: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Provider built-in tools exposed as generic capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuiltinTool {
    WebSearch,
    FileSearch,
    CodeExecution,
    ComputerUse,
    UrlContext,
    Maps,
    Mcp {
        #[serde(skip_serializing_if = "Option::is_none")]
        server_label: Option<String>,
    },
    Vendor {
        name: String,
        #[serde(default, skip_serializing_if = "Value::is_null")]
        payload: Value,
    },
}

/// Structured output / schema constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub schema: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strict: bool,
}

/// Reasoning-related controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Desired output modalities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputModality {
    Text,
    Image,
    Audio,
    Json,
}

/// Safety / policy settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Cache hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Provider-neutral prompt cache policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptCachePolicy {
    Disabled,
    BestEffort {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<PromptCacheKey>,
        #[serde(
            default,
            skip_serializing_if = "PromptCacheRetention::is_provider_default"
        )]
        retention: PromptCacheRetention,
        #[serde(default, skip_serializing_if = "CacheBreakpoint::is_auto")]
        breakpoint: CacheBreakpoint,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        vendor_extensions: VendorExtensions,
    },
    Required {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<PromptCacheKey>,
        #[serde(
            default,
            skip_serializing_if = "PromptCacheRetention::is_provider_default"
        )]
        retention: PromptCacheRetention,
        #[serde(default, skip_serializing_if = "CacheBreakpoint::is_auto")]
        breakpoint: CacheBreakpoint,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        vendor_extensions: VendorExtensions,
    },
}

impl PromptCachePolicy {
    pub fn best_effort() -> Self {
        Self::BestEffort {
            key: None,
            retention: PromptCacheRetention::ProviderDefault,
            breakpoint: CacheBreakpoint::Auto,
            vendor_extensions: VendorExtensions::new(),
        }
    }

    pub fn required() -> Self {
        Self::Required {
            key: None,
            retention: PromptCacheRetention::ProviderDefault,
            breakpoint: CacheBreakpoint::Auto,
            vendor_extensions: VendorExtensions::new(),
        }
    }

    pub fn from_legacy_cache_settings(settings: CacheSettings) -> Self {
        if settings.enabled {
            Self::BestEffort {
                key: None,
                retention: PromptCacheRetention::ProviderDefault,
                breakpoint: CacheBreakpoint::Auto,
                vendor_extensions: settings.vendor_extensions,
            }
        } else {
            Self::Disabled
        }
    }

    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required { .. })
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }

    pub fn key(&self) -> Option<&PromptCacheKey> {
        match self {
            Self::Disabled => None,
            Self::BestEffort { key, .. } | Self::Required { key, .. } => key.as_ref(),
        }
    }

    pub fn retention(&self) -> PromptCacheRetention {
        match self {
            Self::Disabled => PromptCacheRetention::ProviderDefault,
            Self::BestEffort { retention, .. } | Self::Required { retention, .. } => *retention,
        }
    }

    pub fn breakpoint(&self) -> CacheBreakpoint {
        match self {
            Self::Disabled => CacheBreakpoint::Auto,
            Self::BestEffort { breakpoint, .. } | Self::Required { breakpoint, .. } => {
                breakpoint.clone()
            }
        }
    }
}

/// Provider cache key input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptCacheKey {
    Explicit {
        value: String,
    },
    StablePrefixHash {
        namespace: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tenant_scope: Option<String>,
    },
}

/// Provider-neutral cache retention intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PromptCacheRetention {
    #[default]
    ProviderDefault,
    Short,
    Long,
}

impl PromptCacheRetention {
    pub fn is_provider_default(&self) -> bool {
        *self == Self::ProviderDefault
    }
}

/// Cacheable-prefix boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheBreakpoint {
    #[default]
    Auto,
    EndOfTools,
    EndOfInstructions,
    EndOfMessage {
        index: usize,
    },
    EndOfContentBlock {
        message_index: usize,
        part_index: usize,
    },
}

impl CacheBreakpoint {
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

/// Provider-reported prompt cache usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PromptCacheUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_short_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_long_input_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl PromptCacheUsage {
    pub fn is_empty(&self) -> bool {
        self.cached_input_tokens.is_none()
            && self.cache_read_input_tokens.is_none()
            && self.cache_creation_input_tokens.is_none()
            && self.cache_creation_short_input_tokens.is_none()
            && self.cache_creation_long_input_tokens.is_none()
            && self.vendor_extensions.is_empty()
    }
}

fn default_true() -> bool {
    true
}
