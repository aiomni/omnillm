use serde_json::{json, Value};

use crate::types::{LlmResponse, Message, MessageRole, TokenUsage, VendorExtensions};

use super::super::common::{
    assistant_message_from_response, finish_reason_string_upper_camel, parse_finish_reason,
    response_items_from_message,
};
use super::super::{ProtocolError, ProviderProtocol};
use super::helpers::*;

pub(in crate::protocol) fn parse_gemini_response(
    body: &Value,
) -> Result<LlmResponse, ProtocolError> {
    let candidate = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .ok_or_else(|| ProtocolError::MissingField("candidates[0]".into()))?;
    let message = parse_gemini_candidate(candidate)?;
    let output = response_items_from_message(&message);
    Ok(LlmResponse {
        output,
        messages: vec![message.clone()],
        content_text: message.plain_text(),
        usage: TokenUsage {
            prompt_tokens: body
                .get("usageMetadata")
                .and_then(|value| value.get("promptTokenCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            completion_tokens: body
                .get("usageMetadata")
                .and_then(|value| value.get("candidatesTokenCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: body
                .get("usageMetadata")
                .and_then(|value| value.get("totalTokenCount"))
                .and_then(Value::as_u64)
                .map(|value| value as u32),
            prompt_cache: None,
        },
        model: body
            .get("modelVersion")
            .or_else(|| body.get("model"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::GeminiGenerateContent,
        finish_reason: candidate.get("finishReason").and_then(parse_finish_reason),
        response_id: body
            .get("responseId")
            .and_then(Value::as_str)
            .map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol) fn emit_gemini_response(
    response: &LlmResponse,
) -> Result<Value, ProtocolError> {
    let message = assistant_message_from_response(response)
        .unwrap_or_else(|| Message::text(MessageRole::Assistant, response.content_text.clone()));
    Ok(json!({
        "modelVersion": response.model,
        "responseId": response.response_id,
        "candidates": [{
            "finishReason": response.finish_reason.as_ref().map(finish_reason_string_upper_camel),
            "content": gemini_content_json(message)?,
        }],
        "usageMetadata": {
            "promptTokenCount": response.usage.prompt_tokens,
            "candidatesTokenCount": response.usage.completion_tokens,
            "totalTokenCount": response.usage.total(),
        }
    }))
}
