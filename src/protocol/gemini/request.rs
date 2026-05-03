use serde_json::{json, Map, Value};

use crate::types::{LlmRequest, Message, MessageRole, RequestItem, VendorExtensions};

use super::super::common::{parse_generation, request_messages_for_separate_instruction_protocol};
use super::super::ProtocolError;
use super::helpers::*;

pub(in crate::protocol) fn parse_gemini_request(body: &Value) -> Result<LlmRequest, ProtocolError> {
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::MissingField("model".into()))?
        .to_string();

    let instructions = body
        .get("systemInstruction")
        .map(parse_gemini_instruction)
        .transpose()?;
    let mut messages = body
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("contents".into()))?
        .iter()
        .map(parse_gemini_content)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(text) = &instructions {
        messages.insert(0, Message::text(MessageRole::System, text.clone()));
    }
    let input = messages.iter().cloned().map(RequestItem::from).collect();
    let capabilities = parse_gemini_capabilities(body)?;
    let generation_config = body.get("generationConfig");
    let generation = parse_generation(
        generation_config
            .and_then(|value| value.get("maxOutputTokens"))
            .and_then(Value::as_u64),
        generation_config
            .and_then(|value| value.get("temperature"))
            .and_then(Value::as_f64),
        generation_config
            .and_then(|value| value.get("topP"))
            .and_then(Value::as_f64),
        generation_config
            .and_then(|value| value.get("topK"))
            .and_then(Value::as_u64),
        generation_config
            .and_then(|value| value.get("stopSequences"))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        None,
        None,
        generation_config
            .and_then(|value| value.get("seed"))
            .and_then(Value::as_u64),
    );

    Ok(LlmRequest {
        model,
        instructions,
        input,
        messages,
        capabilities,
        generation,
        metadata: VendorExtensions::new(),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol) fn emit_gemini_request(
    request: &LlmRequest,
) -> Result<Value, ProtocolError> {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    emit_gemini_request_inner(request, map)
}

pub(in crate::protocol) fn emit_gemini_transport_request(
    request: &LlmRequest,
) -> Result<Value, ProtocolError> {
    emit_gemini_request_inner(request, Map::new())
}

fn emit_gemini_request_inner(
    request: &LlmRequest,
    mut map: Map<String, Value>,
) -> Result<Value, ProtocolError> {
    let contents = request_messages_for_separate_instruction_protocol(request)
        .into_iter()
        .map(gemini_content_json)
        .collect::<Result<Vec<_>, _>>()?;
    map.insert("contents".into(), Value::Array(contents));

    if let Some(instructions) = request.normalized_instructions() {
        map.insert(
            "systemInstruction".into(),
            json!({
                "role": "system",
                "parts": [{ "text": instructions }],
            }),
        );
    }

    let tools = emit_gemini_tools(&request.capabilities)?;
    if !tools.is_empty() {
        map.insert("tools".into(), Value::Array(tools));
    }

    let mut generation_config = Map::new();
    if let Some(max_tokens) = request.generation.max_output_tokens {
        generation_config.insert("maxOutputTokens".into(), Value::from(max_tokens));
    }
    if let Some(temperature) = request.generation.temperature {
        generation_config.insert("temperature".into(), Value::from(temperature));
    }
    if let Some(top_p) = request.generation.top_p {
        generation_config.insert("topP".into(), Value::from(top_p));
    }
    if let Some(top_k) = request.generation.top_k {
        generation_config.insert("topK".into(), Value::from(top_k));
    }
    if !request.generation.stop_sequences.is_empty() {
        generation_config.insert(
            "stopSequences".into(),
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
    if let Some(seed) = request.generation.seed {
        generation_config.insert("seed".into(), Value::from(seed));
    }
    if let Some(structured_output) = &request.capabilities.structured_output {
        generation_config.insert(
            "responseMimeType".into(),
            Value::String("application/json".into()),
        );
        generation_config.insert("responseSchema".into(), structured_output.schema.clone());
    }
    if !generation_config.is_empty() {
        map.insert("generationConfig".into(), Value::Object(generation_config));
    }

    Ok(Value::Object(map))
}
