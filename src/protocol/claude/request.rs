use serde_json::{Map, Value};

use crate::types::{
    CapabilitySet, LlmRequest, Message, MessageRole, OutputModality, RequestItem, VendorExtensions,
};

use super::super::common::{
    parse_generation, request_messages_for_separate_instruction_protocol, required_str,
    string_or_array,
};
use super::super::ProtocolError;
use super::helpers::*;

pub(in crate::protocol) fn parse_claude_request(body: &Value) -> Result<LlmRequest, ProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let system = match body.get("system") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Array(blocks)) => {
            let joined = blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    };
    let mut messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("messages".into()))?
        .iter()
        .map(parse_claude_message)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(text) = &system {
        messages.insert(0, Message::text(MessageRole::System, text.clone()));
    }
    let input = messages.iter().cloned().map(RequestItem::from).collect();
    let capabilities = CapabilitySet {
        tools: parse_claude_tools(body.get("tools"))?,
        structured_output: None,
        reasoning: None,
        modalities: vec![OutputModality::Text],
        safety: None,
        cache: None,
        prompt_cache: parse_claude_prompt_cache_policy(body),
        builtin_tools: Vec::new(),
        vendor_extensions: VendorExtensions::new(),
    };
    let generation = parse_generation(
        body.get("max_tokens").and_then(Value::as_u64),
        body.get("temperature").and_then(Value::as_f64),
        body.get("top_p").and_then(Value::as_f64),
        None,
        string_or_array(body.get("stop_sequences")),
        None,
        None,
        None,
    );

    Ok(LlmRequest {
        model,
        instructions: system,
        input,
        messages,
        capabilities,
        generation,
        metadata: VendorExtensions::new(),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol) fn emit_claude_request(
    request: &LlmRequest,
    stream: bool,
) -> Result<Value, ProtocolError> {
    if !request.capabilities.builtin_tools.is_empty() {
        return Err(ProtocolError::UnsupportedFeature(
            "builtin tools in Claude Messages".into(),
        ));
    }
    if request.capabilities.structured_output.is_some() {
        return Err(ProtocolError::UnsupportedFeature(
            "structured output in Claude Messages".into(),
        ));
    }
    if request.capabilities.reasoning.is_some() {
        return Err(ProtocolError::UnsupportedFeature(
            "reasoning capability in Claude Messages".into(),
        ));
    }

    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert(
        "max_tokens".into(),
        Value::from(request.generation.max_output_tokens.unwrap_or(1024)),
    );

    let prompt_cache = request.capabilities.effective_prompt_cache();
    let mut system = request.normalized_instructions().map(Value::String);
    let mut messages = request_messages_for_separate_instruction_protocol(request)
        .into_iter()
        .map(claude_message_json)
        .collect::<Result<Vec<_>, _>>()?;
    let mut tools = emit_claude_tools(&request.capabilities.tools);

    apply_claude_prompt_cache_policy(
        &mut tools,
        system.as_mut(),
        &mut messages,
        prompt_cache.as_ref(),
    )?;

    if let Some(system) = system {
        map.insert("system".into(), system);
    }
    map.insert("messages".into(), Value::Array(messages));
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }

    if let Some(temperature) = request.generation.temperature {
        map.insert("temperature".into(), Value::from(temperature));
    }
    if let Some(top_p) = request.generation.top_p {
        map.insert("top_p".into(), Value::from(top_p));
    }
    if !request.generation.stop_sequences.is_empty() {
        map.insert(
            "stop_sequences".into(),
            Value::Array(
                request
                    .generation
                    .stop_sequences
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    Ok(Value::Object(map))
}
