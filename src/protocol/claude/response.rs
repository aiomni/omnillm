use serde_json::{json, Value};

use crate::types::{LlmResponse, Message, MessageRole, TokenUsage, VendorExtensions};

use super::super::common::{
    assistant_message_from_response, finish_reason_string, parse_finish_reason,
    response_items_from_message,
};
use super::super::{ProtocolError, ProviderProtocol};
use super::helpers::*;

pub(in crate::protocol) fn parse_claude_response(
    body: &Value,
) -> Result<LlmResponse, ProtocolError> {
    let message = parse_claude_message(body)?;
    let output = response_items_from_message(&message);
    Ok(LlmResponse {
        output,
        messages: vec![message.clone()],
        content_text: message.plain_text(),
        usage: TokenUsage {
            prompt_tokens: body
                .get("usage")
                .and_then(|value| value.get("input_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            completion_tokens: body
                .get("usage")
                .and_then(|value| value.get("output_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: None,
            prompt_cache: parse_claude_prompt_cache_usage(body.get("usage")),
        },
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::ClaudeMessages,
        finish_reason: body.get("stop_reason").and_then(parse_finish_reason),
        response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol) fn emit_claude_response(
    response: &LlmResponse,
) -> Result<Value, ProtocolError> {
    let message = assistant_message_from_response(response)
        .unwrap_or_else(|| Message::text(MessageRole::Assistant, response.content_text.clone()));
    Ok(json!({
        "id": response.response_id,
        "type": "message",
        "role": "assistant",
        "model": response.model,
        "stop_reason": response.finish_reason.as_ref().map(finish_reason_string),
        "content": claude_content_parts(&message.parts)?,
        "usage": claude_usage_json(&response.usage)
    }))
}
