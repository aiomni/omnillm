use serde_json::{json, Value};

use crate::types::{LlmStreamEvent, TokenUsage};

use super::super::{ProtocolError, ProviderProtocol, ProviderStreamFrame};
use super::helpers::*;

pub(in crate::protocol) fn parse_claude_stream_event(
    frame: &ProviderStreamFrame,
    body: &Value,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    match frame.event.as_deref().unwrap_or_default() {
        "message_start" => Ok(Some(LlmStreamEvent::ResponseStarted {
            response_id: body
                .get("message")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            model: body
                .get("message")
                .and_then(|value| value.get("model"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            provider_protocol: ProviderProtocol::ClaudeMessages,
        })),
        "content_block_delta" => {
            let delta = body
                .get("delta")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if delta.is_empty() {
                Ok(None)
            } else {
                Ok(Some(LlmStreamEvent::TextDelta { delta }))
            }
        }
        "message_delta" => {
            let usage = body.get("usage").map(|usage| TokenUsage {
                prompt_tokens: usage
                    .get("input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                completion_tokens: usage
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                total_tokens: None,
                prompt_cache: parse_claude_prompt_cache_usage(Some(usage)),
            });
            Ok(usage.map(|usage| LlmStreamEvent::Usage { usage }))
        }
        "message_stop" => Ok(None),
        "error" => Ok(Some(LlmStreamEvent::Error {
            message: body
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        })),
        _ => Ok(None),
    }
}

pub(in crate::protocol) fn emit_claude_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let frame = match event {
        LlmStreamEvent::ResponseStarted {
            response_id, model, ..
        } => ProviderStreamFrame {
            event: Some("message_start".into()),
            data: serde_json::to_string(&json!({
                "message": { "id": response_id, "model": model }
            }))?,
        },
        LlmStreamEvent::TextDelta { delta } => ProviderStreamFrame {
            event: Some("content_block_delta".into()),
            data: serde_json::to_string(&json!({
                "delta": { "type": "text_delta", "text": delta }
            }))?,
        },
        LlmStreamEvent::Usage { usage } => ProviderStreamFrame {
            event: Some("message_delta".into()),
            data: serde_json::to_string(&json!({
                "usage": claude_usage_json(usage)
            }))?,
        },
        LlmStreamEvent::Completed { .. } => ProviderStreamFrame {
            event: Some("message_stop".into()),
            data: "{}".into(),
        },
        LlmStreamEvent::Error { message } => ProviderStreamFrame {
            event: Some("error".into()),
            data: serde_json::to_string(&json!({
                "error": { "message": message }
            }))?,
        },
        _ => return Ok(None),
    };
    Ok(Some(frame))
}
