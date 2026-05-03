use serde_json::{json, Map, Value};

use crate::api::{
    EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage, EmbeddingVector,
};

use super::common::*;
use super::ApiProtocolError;

pub(super) fn emit_openai_embeddings_request(
    request: &EmbeddingRequest,
) -> Result<Value, ApiProtocolError> {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert("input".into(), embedding_inputs_to_value(&request.input));
    if let Some(dimensions) = request.dimensions {
        map.insert("dimensions".into(), Value::from(dimensions));
    }
    if let Some(format) = &request.encoding_format {
        map.insert("encoding_format".into(), Value::String(format.clone()));
    }
    if let Some(user) = &request.user {
        map.insert("user".into(), Value::String(user.clone()));
    }
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);
    Ok(Value::Object(map))
}

pub(super) fn parse_openai_embeddings_request(
    body: &Value,
) -> Result<EmbeddingRequest, ApiProtocolError> {
    let model = required_str(body, "model")?.to_string();
    let input = parse_embedding_inputs(
        body.get("input")
            .ok_or_else(|| ApiProtocolError::MissingField("input".into()))?,
    )?;

    Ok(EmbeddingRequest {
        model,
        input,
        dimensions: body
            .get("dimensions")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        encoding_format: body
            .get("encoding_format")
            .and_then(Value::as_str)
            .map(str::to_owned),
        user: body.get("user").and_then(Value::as_str).map(str::to_owned),
        vendor_extensions: collect_vendor_extensions(
            body,
            &["model", "input", "dimensions", "encoding_format", "user"],
        ),
    })
}

pub(super) fn emit_openai_embeddings_response(response: &EmbeddingResponse) -> Value {
    let mut map = Map::new();
    map.insert("object".into(), Value::String("list".into()));
    map.insert("model".into(), Value::String(response.model.clone()));
    map.insert(
        "data".into(),
        Value::Array(
            response
                .data
                .iter()
                .map(|vector| {
                    json!({
                        "object": "embedding",
                        "index": vector.index,
                        "embedding": vector.embedding,
                    })
                })
                .collect(),
        ),
    );
    if let Some(usage) = &response.usage {
        let mut usage_map = Map::new();
        usage_map.insert("prompt_tokens".into(), Value::from(usage.prompt_tokens));
        if let Some(total_tokens) = usage.total_tokens {
            usage_map.insert("total_tokens".into(), Value::from(total_tokens));
        }
        map.insert("usage".into(), Value::Object(usage_map));
    }
    extend_with_vendor_extensions(&mut map, &response.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_embeddings_response(
    body: &Value,
) -> Result<EmbeddingResponse, ApiProtocolError> {
    let data = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiProtocolError::MissingField("data".into()))?
        .iter()
        .map(|item| {
            Ok(EmbeddingVector {
                index: item.get("index").and_then(Value::as_u64).unwrap_or(0) as usize,
                embedding: item
                    .get("embedding")
                    .and_then(Value::as_array)
                    .ok_or_else(|| ApiProtocolError::MissingField("embedding".into()))?
                    .iter()
                    .map(|value| {
                        value.as_f64().map(|number| number as f32).ok_or_else(|| {
                            ApiProtocolError::InvalidShape("embedding vector entry".into())
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, ApiProtocolError>>()?;

    let usage = body.get("usage").map(|usage| EmbeddingUsage {
        prompt_tokens: usage
            .get("prompt_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        total_tokens: usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
    });

    Ok(EmbeddingResponse {
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        data,
        usage,
        vendor_extensions: collect_vendor_extensions(body, &["object", "model", "data", "usage"]),
    })
}

fn embedding_inputs_to_value(inputs: &[EmbeddingInput]) -> Value {
    match inputs {
        [EmbeddingInput::Text { text }] => Value::String(text.clone()),
        [EmbeddingInput::Tokens { tokens }] => {
            Value::Array(tokens.iter().copied().map(Value::from).collect::<Vec<_>>())
        }
        many => Value::Array(
            many.iter()
                .map(|item| match item {
                    EmbeddingInput::Text { text } => Value::String(text.clone()),
                    EmbeddingInput::Tokens { tokens } => {
                        Value::Array(tokens.iter().copied().map(Value::from).collect::<Vec<_>>())
                    }
                })
                .collect(),
        ),
    }
}

fn parse_embedding_inputs(value: &Value) -> Result<Vec<EmbeddingInput>, ApiProtocolError> {
    match value {
        Value::String(text) => Ok(vec![EmbeddingInput::Text { text: text.clone() }]),
        Value::Array(items) => {
            if items.is_empty() {
                return Ok(Vec::new());
            }
            if items.iter().all(Value::is_number) {
                return Ok(vec![EmbeddingInput::Tokens {
                    tokens: items
                        .iter()
                        .map(value_as_i32)
                        .collect::<Result<Vec<_>, _>>()?,
                }]);
            }
            items
                .iter()
                .map(|item| match item {
                    Value::String(text) => Ok(EmbeddingInput::Text { text: text.clone() }),
                    Value::Array(tokens) if tokens.iter().all(Value::is_number) => {
                        Ok(EmbeddingInput::Tokens {
                            tokens: tokens
                                .iter()
                                .map(value_as_i32)
                                .collect::<Result<Vec<_>, _>>()?,
                        })
                    }
                    _ => Err(ApiProtocolError::InvalidShape(
                        "embedding input item".into(),
                    )),
                })
                .collect()
        }
        _ => Err(ApiProtocolError::InvalidShape("embedding input".into())),
    }
}
