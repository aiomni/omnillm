use serde_json::{Map, Value};

use crate::api::{GeneratedImage, ImageGenerationRequest, ImageGenerationResponse};

use super::common::*;
use super::ApiProtocolError;

pub(super) fn emit_openai_image_generation_request(request: &ImageGenerationRequest) -> Value {
    let mut map = Map::new();
    if let Some(model) = &request.model {
        map.insert("model".into(), Value::String(model.clone()));
    }
    map.insert("prompt".into(), Value::String(request.prompt.clone()));
    if let Some(size) = &request.size {
        map.insert("size".into(), Value::String(size.clone()));
    }
    if let Some(quality) = &request.quality {
        map.insert("quality".into(), Value::String(quality.clone()));
    }
    if let Some(style) = &request.style {
        map.insert("style".into(), Value::String(style.clone()));
    }
    if let Some(background) = &request.background {
        map.insert("background".into(), Value::String(background.clone()));
    }
    if let Some(output_format) = &request.output_format {
        map.insert("output_format".into(), Value::String(output_format.clone()));
    }
    if let Some(n) = request.n {
        map.insert("n".into(), Value::from(n));
    }
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_image_generation_request(
    body: &Value,
) -> Result<ImageGenerationRequest, ApiProtocolError> {
    Ok(ImageGenerationRequest {
        model: body.get("model").and_then(Value::as_str).map(str::to_owned),
        prompt: required_str(body, "prompt")?.to_string(),
        size: body.get("size").and_then(Value::as_str).map(str::to_owned),
        quality: body
            .get("quality")
            .and_then(Value::as_str)
            .map(str::to_owned),
        style: body.get("style").and_then(Value::as_str).map(str::to_owned),
        background: body
            .get("background")
            .and_then(Value::as_str)
            .map(str::to_owned),
        output_format: body
            .get("output_format")
            .and_then(Value::as_str)
            .map(str::to_owned),
        n: body
            .get("n")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        vendor_extensions: collect_vendor_extensions(
            body,
            &[
                "model",
                "prompt",
                "size",
                "quality",
                "style",
                "background",
                "output_format",
                "n",
            ],
        ),
    })
}

pub(super) fn emit_openai_image_generation_response(response: &ImageGenerationResponse) -> Value {
    let mut map = Map::new();
    if let Some(created) = response.created {
        map.insert("created".into(), Value::from(created));
    }
    map.insert(
        "data".into(),
        Value::Array(
            response
                .data
                .iter()
                .map(|image| {
                    let mut image_map = Map::new();
                    if let Some(url) = &image.url {
                        image_map.insert("url".into(), Value::String(url.clone()));
                    }
                    if let Some(b64_json) = &image.b64_json {
                        image_map.insert("b64_json".into(), Value::String(b64_json.clone()));
                    }
                    if let Some(revised_prompt) = &image.revised_prompt {
                        image_map.insert(
                            "revised_prompt".into(),
                            Value::String(revised_prompt.clone()),
                        );
                    }
                    if let Some(media_type) = &image.media_type {
                        image_map.insert("media_type".into(), Value::String(media_type.clone()));
                    }
                    Value::Object(image_map)
                })
                .collect(),
        ),
    );
    extend_with_vendor_extensions(&mut map, &response.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_image_generation_response(
    body: &Value,
) -> Result<ImageGenerationResponse, ApiProtocolError> {
    let data = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiProtocolError::MissingField("data".into()))?
        .iter()
        .map(|item| GeneratedImage {
            url: item.get("url").and_then(Value::as_str).map(str::to_owned),
            b64_json: item
                .get("b64_json")
                .and_then(Value::as_str)
                .map(str::to_owned),
            revised_prompt: item
                .get("revised_prompt")
                .and_then(Value::as_str)
                .map(str::to_owned),
            media_type: item
                .get("media_type")
                .or_else(|| item.get("mime_type"))
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
        .collect();

    Ok(ImageGenerationResponse {
        created: body.get("created").and_then(Value::as_u64),
        data,
        vendor_extensions: collect_vendor_extensions(body, &["created", "data"]),
    })
}
