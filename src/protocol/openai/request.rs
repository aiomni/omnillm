use serde_json::{json, Map, Value};

use crate::types::{CapabilitySet, LlmRequest, OutputModality, RequestItem, VendorExtensions};

use super::super::common::{
    chat_messages_with_instructions, collect_vendor_extensions, emit_generation_common,
    extend_with_vendor_extensions, extensions_to_object, object_to_extensions, parse_generation,
    request_items_for_instructionless_protocol, required_str, string_or_array,
};
use super::super::ProtocolError;
use super::helpers::*;

pub(in crate::protocol) fn parse_openai_responses_request(
    body: &Value,
) -> Result<LlmRequest, ProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let instructions = body
        .get("instructions")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let input = parse_openai_responses_input(body.get("input").unwrap_or(&Value::Null))?;
    let messages = input
        .iter()
        .filter_map(RequestItem::as_message)
        .cloned()
        .collect::<Vec<_>>();
    let capabilities = parse_openai_responses_capabilities(body)?;
    let generation = parse_generation(
        body.get("max_output_tokens").and_then(Value::as_u64),
        body.get("temperature").and_then(Value::as_f64),
        body.get("top_p").and_then(Value::as_f64),
        body.get("top_k").and_then(Value::as_u64),
        string_or_array(body.get("stop")),
        None,
        None,
        body.get("seed").and_then(Value::as_u64),
    );

    Ok(LlmRequest {
        model,
        instructions,
        input,
        messages,
        capabilities,
        generation,
        metadata: object_to_extensions(body.get("metadata")),
        vendor_extensions: collect_vendor_extensions(
            body,
            &[
                "model",
                "instructions",
                "input",
                "tools",
                "text",
                "reasoning",
                "max_output_tokens",
                "temperature",
                "top_p",
                "top_k",
                "stop",
                "presence_penalty",
                "frequency_penalty",
                "seed",
                "metadata",
                "prompt_cache_key",
                "prompt_cache_retention",
                "stream",
            ],
        ),
    })
}

pub(in crate::protocol) fn emit_openai_responses_request(
    request: &LlmRequest,
    stream: bool,
) -> Result<Value, ProtocolError> {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));

    if let Some(instructions) = request.normalized_instructions() {
        map.insert("instructions".into(), Value::String(instructions));
    }

    let input = request_items_for_instructionless_protocol(request)
        .into_iter()
        .map(openai_responses_input_item)
        .collect::<Result<Vec<_>, _>>()?;
    if !input.is_empty() {
        map.insert("input".into(), Value::Array(input));
    }

    emit_generation_common(&mut map, &request.generation, true);
    emit_openai_responses_capabilities(&mut map, &request.capabilities)?;
    emit_openai_prompt_cache_policy(&mut map, request)?;

    if !request.metadata.is_empty() {
        map.insert(
            "metadata".into(),
            Value::Object(extensions_to_object(&request.metadata)),
        );
    }
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);
    if stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    Ok(Value::Object(map))
}

pub(in crate::protocol) fn parse_openai_chat_request(
    body: &Value,
) -> Result<LlmRequest, ProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("messages".into()))?
        .iter()
        .map(parse_openai_chat_message)
        .collect::<Result<Vec<_>, _>>()?;
    let input = messages.iter().cloned().map(RequestItem::from).collect();
    let capabilities = CapabilitySet {
        tools: parse_function_tools(body.get("tools"))?,
        structured_output: parse_openai_chat_structured_output(body),
        reasoning: None,
        modalities: vec![OutputModality::Text],
        safety: None,
        cache: None,
        prompt_cache: parse_openai_prompt_cache_policy(body),
        builtin_tools: Vec::new(),
        vendor_extensions: VendorExtensions::new(),
    };
    let generation = parse_generation(
        body.get("max_tokens").and_then(Value::as_u64),
        body.get("temperature").and_then(Value::as_f64),
        body.get("top_p").and_then(Value::as_f64),
        None,
        string_or_array(body.get("stop")),
        body.get("presence_penalty").and_then(Value::as_f64),
        body.get("frequency_penalty").and_then(Value::as_f64),
        body.get("seed").and_then(Value::as_u64),
    );

    Ok(LlmRequest {
        model,
        instructions: None,
        input,
        messages,
        capabilities,
        generation,
        metadata: VendorExtensions::new(),
        vendor_extensions: collect_vendor_extensions(
            body,
            &[
                "model",
                "messages",
                "tools",
                "response_format",
                "max_tokens",
                "temperature",
                "top_p",
                "stop",
                "presence_penalty",
                "frequency_penalty",
                "seed",
                "prompt_cache_key",
                "prompt_cache_retention",
                "stream",
            ],
        ),
    })
}

pub(in crate::protocol) fn emit_openai_chat_request(
    request: &LlmRequest,
    stream: bool,
) -> Result<Value, ProtocolError> {
    if !request.capabilities.builtin_tools.is_empty() {
        return Err(ProtocolError::UnsupportedFeature(
            "builtin tools in OpenAI Chat Completions".into(),
        ));
    }
    if request.capabilities.reasoning.is_some() {
        return Err(ProtocolError::UnsupportedFeature(
            "reasoning capability in OpenAI Chat Completions".into(),
        ));
    }

    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));

    let messages = chat_messages_with_instructions(request)
        .into_iter()
        .map(openai_chat_message_json)
        .collect::<Result<Vec<_>, _>>()?;
    map.insert("messages".into(), Value::Array(messages));

    let tools = emit_function_tools(&request.capabilities.tools);
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }

    if let Some(structured_output) = &request.capabilities.structured_output {
        map.insert(
            "response_format".into(),
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": structured_output
                        .name
                        .clone()
                        .unwrap_or_else(|| "response".into()),
                    "schema": structured_output.schema,
                    "strict": structured_output.strict,
                }
            }),
        );
    }

    emit_generation_common(&mut map, &request.generation, false);
    emit_openai_prompt_cache_policy(&mut map, request)?;
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);

    if stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    Ok(Value::Object(map))
}
