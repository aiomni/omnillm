use serde_json::{json, Value};

use crate::types::{LlmStreamEvent, TokenUsage};

use super::super::{ProtocolError, ProviderProtocol, ProviderStreamFrame};
use super::helpers::*;
use super::response::{emit_openai_responses_response, parse_openai_responses_response};

pub(in crate::protocol) fn parse_openai_responses_stream_event(
    frame: &ProviderStreamFrame,
    body: &Value,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    let event = frame
        .event
        .clone()
        .or_else(|| body.get("type").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default();

    Ok(match event.as_str() {
        "response.created" | "response.in_progress" => Some(LlmStreamEvent::ResponseStarted {
            response_id: body
                .get("response")
                .and_then(|v| v.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| body.get("id").and_then(Value::as_str).map(str::to_owned)),
            model: body
                .get("response")
                .and_then(|v| v.get("model"))
                .or_else(|| body.get("model"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            provider_protocol: ProviderProtocol::OpenAiResponses,
        }),
        "response.output_text.delta" => Some(LlmStreamEvent::TextDelta {
            delta: body
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        "response.function_call_arguments.delta" => Some(LlmStreamEvent::ToolCallDelta {
            call_id: body
                .get("item_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            name: body
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            delta: body
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        "response.output_item.added" => body
            .get("item")
            .map(parse_openai_responses_single_output_item)
            .transpose()?
            .map(|item| LlmStreamEvent::OutputItemAdded { item }),
        "response.reasoning_summary_text.delta" => Some(LlmStreamEvent::ReasoningDelta {
            delta: body
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        "response.completed" => {
            let response = body.get("response").unwrap_or(body);
            Some(LlmStreamEvent::Completed {
                response: parse_openai_responses_response(response)?,
            })
        }
        "response.error" | "error" => Some(LlmStreamEvent::Error {
            message: body
                .get("message")
                .or_else(|| body.get("error").and_then(|value| value.get("message")))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        }),
        _ => None,
    })
}

pub(in crate::protocol) fn emit_openai_responses_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let frame = match event {
        LlmStreamEvent::ResponseStarted {
            response_id, model, ..
        } => ProviderStreamFrame {
            event: Some("response.created".into()),
            data: serde_json::to_string(&json!({
                "type": "response.created",
                "response": { "id": response_id, "model": model }
            }))?,
        },
        LlmStreamEvent::TextDelta { delta } => ProviderStreamFrame {
            event: Some("response.output_text.delta".into()),
            data: serde_json::to_string(&json!({
                "type": "response.output_text.delta",
                "delta": delta
            }))?,
        },
        LlmStreamEvent::ToolCallDelta {
            call_id,
            name,
            delta,
        } => ProviderStreamFrame {
            event: Some("response.function_call_arguments.delta".into()),
            data: serde_json::to_string(&json!({
                "type": "response.function_call_arguments.delta",
                "item_id": call_id,
                "name": name,
                "delta": delta
            }))?,
        },
        LlmStreamEvent::OutputItemAdded { item } => ProviderStreamFrame {
            event: Some("response.output_item.added".into()),
            data: serde_json::to_string(&json!({
                "type": "response.output_item.added",
                "item": openai_responses_output_item(item.clone())?
            }))?,
        },
        LlmStreamEvent::ReasoningDelta { delta } => ProviderStreamFrame {
            event: Some("response.reasoning_summary_text.delta".into()),
            data: serde_json::to_string(&json!({
                "type": "response.reasoning_summary_text.delta",
                "delta": delta
            }))?,
        },
        LlmStreamEvent::Completed { response } => ProviderStreamFrame {
            event: Some("response.completed".into()),
            data: serde_json::to_string(&json!({
                "type": "response.completed",
                "response": emit_openai_responses_response(response)?
            }))?,
        },
        LlmStreamEvent::Error { message } => ProviderStreamFrame {
            event: Some("response.error".into()),
            data: serde_json::to_string(&json!({ "type": "response.error", "message": message }))?,
        },
        LlmStreamEvent::Usage { .. }
        | LlmStreamEvent::ContentPartAdded { .. }
        | LlmStreamEvent::ToolResult { .. } => return Ok(None),
    };
    Ok(Some(frame))
}

pub(in crate::protocol) fn parse_openai_chat_stream_events(
    body: &Value,
) -> Result<Vec<LlmStreamEvent>, ProtocolError> {
    let mut events = Vec::new();

    if let Some(choice) = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
    {
        if let Some(role) = choice
            .get("delta")
            .and_then(|value| value.get("role"))
            .and_then(Value::as_str)
        {
            events.push(LlmStreamEvent::ResponseStarted {
                response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
                model: body
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(role)
                    .to_string(),
                provider_protocol: ProviderProtocol::OpenAiChatCompletions,
            });
        }

        if let Some(delta) = choice
            .get("delta")
            .and_then(|value| value.get("content"))
            .and_then(Value::as_str)
            .filter(|delta| !delta.is_empty())
        {
            events.push(LlmStreamEvent::TextDelta {
                delta: delta.to_string(),
            });
        }

        if let Some(tool_calls) = choice
            .get("delta")
            .and_then(|value| value.get("tool_calls"))
            .and_then(Value::as_array)
        {
            for tool_call in tool_calls {
                events.push(LlmStreamEvent::ToolCallDelta {
                    call_id: tool_call
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: tool_call
                        .get("function")
                        .and_then(|value| value.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    delta: tool_call
                        .get("function")
                        .and_then(|value| value.get("arguments"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }

    if let Some(usage) = body.get("usage") {
        events.push(LlmStreamEvent::Usage {
            usage: TokenUsage {
                prompt_tokens: usage
                    .get("prompt_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                completion_tokens: usage
                    .get("completion_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                total_tokens: usage
                    .get("total_tokens")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                prompt_cache: parse_openai_prompt_cache_usage(Some(usage)),
            },
        });
    }

    Ok(events)
}

pub(in crate::protocol) fn emit_openai_chat_stream_event(
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let data = match event {
        LlmStreamEvent::ResponseStarted {
            response_id, model, ..
        } => json!({
            "id": response_id,
            "model": model,
            "choices": [{ "index": 0, "delta": { "role": "assistant" } }]
        }),
        LlmStreamEvent::TextDelta { delta } => json!({
            "choices": [{ "index": 0, "delta": { "content": delta } }]
        }),
        LlmStreamEvent::ToolCallDelta {
            call_id,
            name,
            delta,
        } => json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "id": call_id,
                        "type": "function",
                        "function": { "name": name, "arguments": delta }
                    }]
                }
            }]
        }),
        LlmStreamEvent::Usage { usage } => json!({
            "usage": openai_chat_usage_json(usage)
        }),
        LlmStreamEvent::Completed { .. } => {
            return Ok(Some(ProviderStreamFrame {
                event: None,
                data: "[DONE]".into(),
            }))
        }
        _ => return Ok(None),
    };

    Ok(Some(ProviderStreamFrame {
        event: None,
        data: serde_json::to_string(&data)?,
    }))
}
