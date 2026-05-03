use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::Value;

use crate::api::ResponseBody;
use crate::types::{PromptCacheUsage, TokenUsage};

use super::{
    PrimitiveAsyncJobStatus, PrimitiveProviderError, PrimitiveProviderKind, PrimitiveResponse,
    PrimitiveUsageTelemetry, ProviderPrimitiveWireFormat,
};

pub(crate) fn extract_usage(
    wire_format: ProviderPrimitiveWireFormat,
    body: &ResponseBody,
) -> Option<PrimitiveUsageTelemetry> {
    let ResponseBody::Json { value } = body else {
        return None;
    };

    let usage = match wire_format {
        ProviderPrimitiveWireFormat::OpenAiResponses
        | ProviderPrimitiveWireFormat::OpenAiChatCompletions
        | ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions => value.get("usage"),
        ProviderPrimitiveWireFormat::OpenAiRealtime => value
            .get("usage")
            .or_else(|| value.pointer("/response/usage")),
        ProviderPrimitiveWireFormat::AnthropicMessages => value.get("usage"),
        ProviderPrimitiveWireFormat::GeminiGenerateContent
        | ProviderPrimitiveWireFormat::GeminiStreamGenerateContent
        | ProviderPrimitiveWireFormat::GeminiLive => value
            .get("usageMetadata")
            .or_else(|| value.pointer("/serverContent/usageMetadata")),
        _ => value.get("usage"),
    }?;

    let token_usage = token_usage_from_raw(wire_format, usage);
    Some(PrimitiveUsageTelemetry {
        raw_usage: usage.clone(),
        token_usage,
        billable_units: Vec::new(),
        vendor_extensions: BTreeMap::new(),
    })
}

pub(crate) fn primitive_error_from_body(
    provider: PrimitiveProviderKind,
    wire_format: ProviderPrimitiveWireFormat,
    status: Option<u16>,
    retry_after: Option<Duration>,
    raw_body: String,
) -> PrimitiveProviderError {
    let parsed = serde_json::from_str::<Value>(&raw_body).ok();
    let code = parsed.as_ref().and_then(extract_error_code);
    let message = parsed
        .as_ref()
        .and_then(extract_error_message)
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| raw_body.clone());

    PrimitiveProviderError {
        provider,
        wire_format,
        status,
        code,
        message,
        retry_after,
        raw_body: Some(raw_body),
        vendor_extensions: BTreeMap::new(),
    }
}

pub(crate) fn extract_async_job_id(response: &PrimitiveResponse) -> Option<String> {
    let ResponseBody::Json { value } = &response.body else {
        return None;
    };
    value
        .get("id")
        .or_else(|| value.get("name"))
        .or_else(|| value.get("batch_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub(crate) fn extract_async_job_status(response: &PrimitiveResponse) -> PrimitiveAsyncJobStatus {
    let ResponseBody::Json { value } = &response.body else {
        return PrimitiveAsyncJobStatus::Unknown;
    };
    let status = value
        .get("status")
        .or_else(|| value.get("state"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    match status.as_str() {
        "pending" | "queued" | "validating" => PrimitiveAsyncJobStatus::Pending,
        "running" | "in_progress" | "processing" => PrimitiveAsyncJobStatus::Running,
        "succeeded" | "completed" | "ended" | "done" => PrimitiveAsyncJobStatus::Succeeded,
        "failed" | "errored" | "expired" => PrimitiveAsyncJobStatus::Failed,
        "cancelled" | "canceled" | "cancelling" | "canceling" => PrimitiveAsyncJobStatus::Cancelled,
        _ => PrimitiveAsyncJobStatus::Unknown,
    }
}

fn token_usage_from_raw(
    wire_format: ProviderPrimitiveWireFormat,
    usage: &Value,
) -> Option<TokenUsage> {
    match wire_format {
        ProviderPrimitiveWireFormat::OpenAiResponses => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["input_tokens"]),
            completion_tokens: usage_u32(usage, &["output_tokens"]),
            total_tokens: usage_u32_opt(usage, &["total_tokens"]),
            prompt_cache: openai_prompt_cache_usage(usage),
        }),
        ProviderPrimitiveWireFormat::OpenAiChatCompletions
        | ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["prompt_tokens"]),
            completion_tokens: usage_u32(usage, &["completion_tokens"]),
            total_tokens: usage_u32_opt(usage, &["total_tokens"]),
            prompt_cache: openai_prompt_cache_usage(usage),
        }),
        ProviderPrimitiveWireFormat::AnthropicMessages => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["input_tokens"]),
            completion_tokens: usage_u32(usage, &["output_tokens"]),
            total_tokens: None,
            prompt_cache: anthropic_prompt_cache_usage(usage),
        }),
        ProviderPrimitiveWireFormat::GeminiGenerateContent
        | ProviderPrimitiveWireFormat::GeminiStreamGenerateContent => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["promptTokenCount"]),
            completion_tokens: usage_u32(usage, &["candidatesTokenCount"]),
            total_tokens: usage_u32_opt(usage, &["totalTokenCount"]),
            prompt_cache: None,
        }),
        _ => generic_token_usage(usage),
    }
}

fn generic_token_usage(usage: &Value) -> Option<TokenUsage> {
    let prompt_tokens = usage_u32(
        usage,
        &[
            "input_tokens",
            "prompt_tokens",
            "promptTokenCount",
            "total_tokens",
        ],
    );
    let completion_tokens = usage_u32(
        usage,
        &["output_tokens", "completion_tokens", "candidatesTokenCount"],
    );
    if prompt_tokens == 0 && completion_tokens == 0 {
        return None;
    }
    Some(TokenUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens: usage_u32_opt(usage, &["total_tokens", "totalTokenCount"]),
        prompt_cache: None,
    })
}

fn openai_prompt_cache_usage(usage: &Value) -> Option<PromptCacheUsage> {
    let cached_input_tokens = usage
        .pointer("/input_tokens_details/cached_tokens")
        .or_else(|| usage.pointer("/prompt_tokens_details/cached_tokens"))
        .and_then(value_to_u32);
    cached_input_tokens.map(|cached_input_tokens| PromptCacheUsage {
        cached_input_tokens: Some(cached_input_tokens),
        ..Default::default()
    })
}

fn anthropic_prompt_cache_usage(usage: &Value) -> Option<PromptCacheUsage> {
    let prompt_cache = PromptCacheUsage {
        cache_read_input_tokens: usage.get("cache_read_input_tokens").and_then(value_to_u32),
        cache_creation_input_tokens: usage
            .get("cache_creation_input_tokens")
            .and_then(value_to_u32),
        cache_creation_short_input_tokens: usage
            .get("cache_creation_5m_input_tokens")
            .or_else(|| usage.pointer("/cache_creation/ephemeral_5m_input_tokens"))
            .and_then(value_to_u32),
        cache_creation_long_input_tokens: usage
            .get("cache_creation_1h_input_tokens")
            .or_else(|| usage.pointer("/cache_creation/ephemeral_1h_input_tokens"))
            .and_then(value_to_u32),
        ..Default::default()
    };
    if prompt_cache.cached_input_tokens.is_some()
        || prompt_cache.cache_read_input_tokens.is_some()
        || prompt_cache.cache_creation_input_tokens.is_some()
        || prompt_cache.cache_creation_short_input_tokens.is_some()
        || prompt_cache.cache_creation_long_input_tokens.is_some()
    {
        Some(prompt_cache)
    } else {
        None
    }
}

fn usage_u32(usage: &Value, fields: &[&str]) -> u32 {
    usage_u32_opt(usage, fields).unwrap_or(0)
}

fn usage_u32_opt(usage: &Value, fields: &[&str]) -> Option<u32> {
    fields
        .iter()
        .find_map(|field| usage.get(*field).and_then(value_to_u32))
}

fn value_to_u32(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

fn extract_error_code(value: &Value) -> Option<String> {
    value
        .pointer("/error/code")
        .or_else(|| value.pointer("/error/type"))
        .or_else(|| value.get("code"))
        .or_else(|| value.get("type"))
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
}

fn extract_error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}
