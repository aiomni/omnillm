use serde_json::{json, Map, Value};

use crate::types::{
    CacheBreakpoint, LlmRequest, PromptCacheKey, PromptCachePolicy, PromptCacheRetention,
    PromptCacheUsage, TokenUsage, VendorExtensions,
};

use super::super::super::common::{
    extend_with_vendor_extensions, nested_u32, prompt_cache_key_value,
    prompt_cache_vendor_extensions,
};
use super::super::super::ProtocolError;

pub(in crate::protocol::openai) fn emit_openai_prompt_cache_policy(
    map: &mut Map<String, Value>,
    request: &LlmRequest,
) -> Result<(), ProtocolError> {
    let Some(policy) = request.capabilities.effective_prompt_cache() else {
        return Ok(());
    };
    if policy.is_disabled() {
        return Ok(());
    }

    if !policy.breakpoint().is_auto() && policy.is_required() {
        return Err(ProtocolError::UnsupportedFeature(
            "OpenAI prompt cache does not support explicit breakpoints".into(),
        ));
    }

    if let Some(key) = policy.key() {
        map.insert(
            "prompt_cache_key".into(),
            Value::String(prompt_cache_key_value(key, request)),
        );
    }

    match policy.retention() {
        PromptCacheRetention::ProviderDefault => {}
        PromptCacheRetention::Short => {
            map.insert(
                "prompt_cache_retention".into(),
                Value::String("in_memory".into()),
            );
        }
        PromptCacheRetention::Long => {
            map.insert("prompt_cache_retention".into(), Value::String("24h".into()));
        }
    }

    extend_with_vendor_extensions(map, prompt_cache_vendor_extensions(&policy));
    Ok(())
}

pub(in crate::protocol::openai) fn parse_openai_prompt_cache_policy(
    body: &Value,
) -> Option<PromptCachePolicy> {
    let key = body
        .get("prompt_cache_key")
        .and_then(Value::as_str)
        .map(|value| PromptCacheKey::Explicit {
            value: value.to_string(),
        });
    let retention = body
        .get("prompt_cache_retention")
        .and_then(Value::as_str)
        .map(parse_openai_prompt_cache_retention)
        .unwrap_or_default();

    if key.is_none() && retention == PromptCacheRetention::ProviderDefault {
        None
    } else {
        Some(PromptCachePolicy::BestEffort {
            key,
            retention,
            breakpoint: CacheBreakpoint::Auto,
            vendor_extensions: VendorExtensions::new(),
        })
    }
}

pub(in crate::protocol::openai) fn parse_openai_prompt_cache_retention(
    value: &str,
) -> PromptCacheRetention {
    match value {
        "24h" => PromptCacheRetention::Long,
        "in_memory" => PromptCacheRetention::Short,
        _ => PromptCacheRetention::ProviderDefault,
    }
}

pub(in crate::protocol::openai) fn parse_openai_prompt_cache_usage(
    usage: Option<&Value>,
) -> Option<PromptCacheUsage> {
    let usage = usage?;
    let cached_input_tokens = nested_u32(usage, &["input_tokens_details", "cached_tokens"])
        .or_else(|| nested_u32(usage, &["prompt_tokens_details", "cached_tokens"]));

    let prompt_cache = PromptCacheUsage {
        cached_input_tokens,
        ..Default::default()
    };
    (!prompt_cache.is_empty()).then_some(prompt_cache)
}

pub(in crate::protocol::openai) fn openai_responses_usage_json(usage: &TokenUsage) -> Value {
    let mut map = Map::new();
    map.insert("input_tokens".into(), Value::from(usage.prompt_tokens));
    map.insert("output_tokens".into(), Value::from(usage.completion_tokens));
    map.insert("total_tokens".into(), Value::from(usage.total()));
    if let Some(cached_tokens) = usage
        .prompt_cache
        .as_ref()
        .and_then(|prompt_cache| prompt_cache.cached_input_tokens)
    {
        map.insert(
            "input_tokens_details".into(),
            json!({ "cached_tokens": cached_tokens }),
        );
    }
    Value::Object(map)
}

pub(in crate::protocol::openai) fn openai_chat_usage_json(usage: &TokenUsage) -> Value {
    let mut map = Map::new();
    map.insert("prompt_tokens".into(), Value::from(usage.prompt_tokens));
    map.insert(
        "completion_tokens".into(),
        Value::from(usage.completion_tokens),
    );
    map.insert("total_tokens".into(), Value::from(usage.total()));
    if let Some(cached_tokens) = usage
        .prompt_cache
        .as_ref()
        .and_then(|prompt_cache| prompt_cache.cached_input_tokens)
    {
        map.insert(
            "prompt_tokens_details".into(),
            json!({ "cached_tokens": cached_tokens }),
        );
    }
    Value::Object(map)
}
