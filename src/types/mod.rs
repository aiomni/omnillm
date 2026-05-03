//! Public canonical request/response types used by [`crate::Gateway`].

use std::collections::BTreeMap;

use serde_json::Value;

mod capabilities;
mod generation;
mod message;
mod prompt_layout;
mod request;
mod stream;

pub use capabilities::{
    BuiltinTool, CacheBreakpoint, CacheSettings, CapabilitySet, OutputModality, PromptCacheKey,
    PromptCachePolicy, PromptCacheRetention, PromptCacheUsage, ReasoningCapability, SafetySettings,
    StructuredOutputConfig, ToolDefinition,
};
pub use generation::{FinishReason, GenerationConfig, LlmResponse, ResponseItem, TokenUsage};
pub use message::{Message, MessagePart, MessageRole, ToolCallPart, ToolResultPart};
pub use prompt_layout::PromptLayoutBuilder;
pub use request::{LlmRequest, RequestItem};
pub use stream::LlmStreamEvent;

/// Arbitrary provider-specific extension payload.
pub type VendorExtensions = BTreeMap<String, Value>;

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt_cache_key(request: &LlmRequest) -> String {
        let Some(PromptCachePolicy::BestEffort {
            key: Some(PromptCacheKey::Explicit { value }),
            ..
        }) = request.capabilities.prompt_cache.as_ref()
        else {
            panic!("expected generated explicit best-effort prompt cache key");
        };
        value.clone()
    }

    #[test]
    fn capability_set_empty_tracks_prompt_cache_policy() {
        let mut capabilities = CapabilitySet::default();
        assert!(capabilities.is_empty());

        capabilities.prompt_cache = Some(PromptCachePolicy::best_effort());
        assert!(!capabilities.is_empty());
    }

    #[test]
    fn legacy_cache_settings_migrate_to_effective_prompt_cache() {
        let disabled = CapabilitySet {
            cache: Some(CacheSettings {
                enabled: false,
                vendor_extensions: VendorExtensions::new(),
            }),
            ..Default::default()
        };
        assert!(matches!(
            disabled.effective_prompt_cache(),
            Some(PromptCachePolicy::Disabled)
        ));

        let enabled = CapabilitySet {
            cache: Some(CacheSettings {
                enabled: true,
                vendor_extensions: [("provider_hint".into(), Value::Bool(true))]
                    .into_iter()
                    .collect(),
            }),
            ..Default::default()
        };
        let Some(PromptCachePolicy::BestEffort {
            vendor_extensions, ..
        }) = enabled.effective_prompt_cache()
        else {
            panic!("expected legacy cache settings to become best-effort prompt cache");
        };
        assert_eq!(
            vendor_extensions.get("provider_hint"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn prompt_cache_policy_serde_round_trips() {
        let policy = PromptCachePolicy::Required {
            key: Some(PromptCacheKey::StablePrefixHash {
                namespace: "docs".into(),
                tenant_scope: Some("tenant-a".into()),
            }),
            retention: PromptCacheRetention::Long,
            breakpoint: CacheBreakpoint::EndOfInstructions,
            vendor_extensions: VendorExtensions::new(),
        };

        let raw = serde_json::to_string(&policy).expect("serialize policy");
        let parsed: PromptCachePolicy = serde_json::from_str(&raw).expect("deserialize policy");
        assert_eq!(parsed, policy);
    }

    #[test]
    fn prompt_layout_builder_key_ignores_dynamic_suffix() {
        let first = PromptLayoutBuilder::new("gpt-5.4")
            .instructions("Use the policy document.")
            .stable_message(Message::text(MessageRole::User, "Stable example"))
            .user_input("Question one")
            .stable_prefix_cache_key(
                "policy",
                Some("tenant-a"),
                PromptCacheRetention::Long,
                false,
            )
            .build();
        let second = PromptLayoutBuilder::new("gpt-5.4")
            .instructions("Use the policy document.")
            .stable_message(Message::text(MessageRole::User, "Stable example"))
            .user_input("Question two")
            .stable_prefix_cache_key(
                "policy",
                Some("tenant-a"),
                PromptCacheRetention::Long,
                false,
            )
            .build();
        let changed_stable = PromptLayoutBuilder::new("gpt-5.4")
            .instructions("Use the updated policy document.")
            .stable_message(Message::text(MessageRole::User, "Stable example"))
            .user_input("Question one")
            .stable_prefix_cache_key(
                "policy",
                Some("tenant-a"),
                PromptCacheRetention::Long,
                false,
            )
            .build();

        assert_eq!(prompt_cache_key(&first), prompt_cache_key(&second));
        assert_ne!(prompt_cache_key(&first), prompt_cache_key(&changed_stable));
        assert_eq!(first.messages.len(), 2);
        assert_eq!(first.messages[0].plain_text(), "Stable example");
        assert_eq!(first.messages[1].plain_text(), "Question one");
    }
}
