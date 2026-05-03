use serde_json::{json, Value};

use crate::types::{LlmStreamEvent, TokenUsage};

use super::super::{ProtocolError, ProviderStreamFrame};
use super::helpers::*;
use super::response::emit_gemini_response;

pub(in crate::protocol) fn parse_gemini_stream_event(
    body: &Value,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    if let Some(candidate) = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
    {
        let message = parse_gemini_candidate(candidate)?;
        let text = message.plain_text();
        if !text.is_empty() {
            return Ok(Some(LlmStreamEvent::TextDelta { delta: text }));
        }
    }

    if let Some(usage) = body.get("usageMetadata") {
        return Ok(Some(LlmStreamEvent::Usage {
            usage: TokenUsage {
                prompt_tokens: usage
                    .get("promptTokenCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                completion_tokens: usage
                    .get("candidatesTokenCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                total_tokens: usage
                    .get("totalTokenCount")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                prompt_cache: None,
            },
        }));
    }

    Ok(None)
}

pub(in crate::protocol) fn emit_gemini_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let data = match event {
        LlmStreamEvent::TextDelta { delta } => json!({
            "candidates": [{
                "content": { "role": "model", "parts": [{ "text": delta }] }
            }]
        }),
        LlmStreamEvent::Usage { usage } => json!({
            "usageMetadata": {
                "promptTokenCount": usage.prompt_tokens,
                "candidatesTokenCount": usage.completion_tokens,
                "totalTokenCount": usage.total(),
            }
        }),
        LlmStreamEvent::Completed { response } => emit_gemini_response(response)?,
        _ => return Ok(None),
    };
    Ok(Some(ProviderStreamFrame {
        event: None,
        data: serde_json::to_string(&data)?,
    }))
}
