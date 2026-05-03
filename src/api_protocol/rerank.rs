use serde_json::{Map, Value};

use crate::api::{RerankDocument, RerankRequest, RerankResponse, RerankResult, RerankUsage};

use super::common::*;
use super::ApiProtocolError;

pub(super) fn emit_openai_rerank_request(request: &RerankRequest) -> Value {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert("query".into(), Value::String(request.query.clone()));
    map.insert(
        "documents".into(),
        Value::Array(
            request
                .documents
                .iter()
                .map(|document| match document {
                    RerankDocument::Text { text } => Value::String(text.clone()),
                    RerankDocument::Json { value } => value.clone(),
                })
                .collect(),
        ),
    );
    if let Some(top_n) = request.top_n {
        map.insert("top_n".into(), Value::from(top_n));
    }
    if let Some(return_documents) = request.return_documents {
        map.insert("return_documents".into(), Value::Bool(return_documents));
    }
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_rerank_request(body: &Value) -> Result<RerankRequest, ApiProtocolError> {
    let documents = body
        .get("documents")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiProtocolError::MissingField("documents".into()))?
        .iter()
        .map(|document| match document {
            Value::String(text) => RerankDocument::Text { text: text.clone() },
            value => RerankDocument::Json {
                value: value.clone(),
            },
        })
        .collect();

    Ok(RerankRequest {
        model: required_str(body, "model")?.to_string(),
        query: required_str(body, "query")?.to_string(),
        documents,
        top_n: body
            .get("top_n")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        return_documents: body.get("return_documents").and_then(Value::as_bool),
        vendor_extensions: collect_vendor_extensions(
            body,
            &["model", "query", "documents", "top_n", "return_documents"],
        ),
    })
}

pub(super) fn emit_openai_rerank_response(response: &RerankResponse) -> Value {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(response.model.clone()));
    map.insert(
        "results".into(),
        Value::Array(
            response
                .results
                .iter()
                .map(|result| {
                    let mut result_map = Map::new();
                    result_map.insert("index".into(), Value::from(result.index));
                    result_map.insert(
                        "relevance_score".into(),
                        Value::from(result.relevance_score),
                    );
                    if let Some(document) = &result.document {
                        result_map.insert("document".into(), document.clone());
                    }
                    Value::Object(result_map)
                })
                .collect(),
        ),
    );
    if let Some(usage) = &response.usage {
        let mut usage_map = Map::new();
        if let Some(total_tokens) = usage.total_tokens {
            usage_map.insert("total_tokens".into(), Value::from(total_tokens));
        }
        map.insert("usage".into(), Value::Object(usage_map));
    }
    extend_with_vendor_extensions(&mut map, &response.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_rerank_response(
    body: &Value,
) -> Result<RerankResponse, ApiProtocolError> {
    let results = body
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiProtocolError::MissingField("results".into()))?
        .iter()
        .map(|result| {
            Ok(RerankResult {
                index: result.get("index").and_then(Value::as_u64).unwrap_or(0) as u32,
                relevance_score: result
                    .get("relevance_score")
                    .and_then(Value::as_f64)
                    .unwrap_or_default() as f32,
                document: result.get("document").cloned(),
            })
        })
        .collect::<Result<Vec<_>, ApiProtocolError>>()?;

    let usage = body.get("usage").map(|usage| RerankUsage {
        total_tokens: usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
    });

    Ok(RerankResponse {
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        results,
        usage,
        vendor_extensions: collect_vendor_extensions(body, &["model", "results", "usage"]),
    })
}
