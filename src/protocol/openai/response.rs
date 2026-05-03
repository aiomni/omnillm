use serde_json::{json, Value};

use crate::types::{LlmResponse, Message, MessageRole, ResponseItem, TokenUsage, VendorExtensions};

use super::super::common::{
    assistant_message_from_response, collect_message_text, finish_reason_string,
    parse_finish_reason, response_item_as_message, response_items_from_message,
    response_output_items,
};
use super::super::{ProtocolError, ProviderProtocol};
use super::helpers::*;

pub(in crate::protocol) fn parse_openai_responses_response(
    body: &Value,
) -> Result<LlmResponse, ProtocolError> {
    let output: Vec<ResponseItem> = body
        .get("output")
        .and_then(Value::as_array)
        .map(|items| parse_openai_responses_output(items))
        .transpose()?
        .unwrap_or_default();
    let messages = output
        .iter()
        .filter_map(response_item_as_message)
        .cloned()
        .collect::<Vec<_>>();
    let content_text = body
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| collect_message_text(&messages));
    let usage = TokenUsage {
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
        total_tokens: body
            .get("usage")
            .and_then(|value| value.get("total_tokens"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        prompt_cache: parse_openai_prompt_cache_usage(body.get("usage")),
    };

    Ok(LlmResponse {
        output,
        messages,
        content_text,
        usage,
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::OpenAiResponses,
        finish_reason: body.get("status").and_then(parse_finish_reason),
        response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol) fn emit_openai_responses_response(
    response: &LlmResponse,
) -> Result<Value, ProtocolError> {
    let output = response_output_items(response)
        .into_iter()
        .map(openai_responses_output_item)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "id": response.response_id,
        "model": response.model,
        "status": response.finish_reason.as_ref().map(finish_reason_string),
        "output_text": response.content_text,
        "output": output,
        "usage": openai_responses_usage_json(&response.usage)
    }))
}

pub(in crate::protocol) fn parse_openai_chat_response(
    body: &Value,
) -> Result<LlmResponse, ProtocolError> {
    let choice = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| ProtocolError::MissingField("choices[0]".into()))?;
    let message = parse_openai_chat_message(choice.get("message").unwrap_or(&Value::Null))?;
    let mut output = response_items_from_message(&message);
    if output.is_empty() {
        output.push(ResponseItem::Message {
            message: message.clone(),
        });
    }
    Ok(LlmResponse {
        output,
        messages: vec![message.clone()],
        content_text: message.plain_text(),
        usage: TokenUsage {
            prompt_tokens: body
                .get("usage")
                .and_then(|value| value.get("prompt_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            completion_tokens: body
                .get("usage")
                .and_then(|value| value.get("completion_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: body
                .get("usage")
                .and_then(|value| value.get("total_tokens"))
                .and_then(Value::as_u64)
                .map(|value| value as u32),
            prompt_cache: parse_openai_prompt_cache_usage(body.get("usage")),
        },
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_protocol: ProviderProtocol::OpenAiChatCompletions,
        finish_reason: choice.get("finish_reason").and_then(parse_finish_reason),
        response_id: body.get("id").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol) fn emit_openai_chat_response(
    response: &LlmResponse,
) -> Result<Value, ProtocolError> {
    let message = assistant_message_from_response(response)
        .unwrap_or_else(|| Message::text(MessageRole::Assistant, response.content_text.clone()));
    Ok(json!({
        "id": response.response_id,
        "model": response.model,
        "choices": [{
            "index": 0,
            "finish_reason": response.finish_reason.as_ref().map(finish_reason_string),
            "message": openai_chat_message_json(message)?,
        }],
        "usage": openai_chat_usage_json(&response.usage)
    }))
}
