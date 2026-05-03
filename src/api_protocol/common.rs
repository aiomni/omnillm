use serde_json::{Map, Value};

use crate::api::{ResponseBody, TransportResponse, WireFormat};
use crate::types::VendorExtensions;

use super::ApiProtocolError;

pub(super) fn json_content_headers() -> std::collections::BTreeMap<String, String> {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    headers
}

pub(super) fn json_response_body(response: &TransportResponse) -> Result<&Value, ApiProtocolError> {
    match &response.body {
        ResponseBody::Json { value } => Ok(value),
        _ => Err(ApiProtocolError::InvalidShape(
            "expected JSON response body".into(),
        )),
    }
}

pub(super) fn audio_media_type_for_format(format: Option<&str>) -> String {
    match format.unwrap_or("mp3") {
        "wav" => "audio/wav",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "opus" => "audio/opus",
        "pcm" => "audio/pcm",
        _ => "audio/mpeg",
    }
    .into()
}

pub(super) fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiProtocolError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiProtocolError::MissingField(field.into()))
}

pub(super) fn value_as_i32(value: &Value) -> Result<i32, ApiProtocolError> {
    value
        .as_i64()
        .and_then(|number| i32::try_from(number).ok())
        .ok_or_else(|| ApiProtocolError::InvalidShape("expected integer token value".into()))
}

pub(super) fn collect_vendor_extensions(value: &Value, known_fields: &[&str]) -> VendorExtensions {
    let Some(object) = value.as_object() else {
        return VendorExtensions::new();
    };

    object
        .iter()
        .filter(|(key, _)| !known_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub(super) fn extend_with_vendor_extensions(
    map: &mut Map<String, Value>,
    vendor_extensions: &VendorExtensions,
) {
    for (key, value) in vendor_extensions {
        map.entry(key.clone()).or_insert_with(|| value.clone());
    }
}

pub(super) fn wire_format_name(wire_format: WireFormat) -> &'static str {
    match wire_format {
        WireFormat::OpenAiResponses => "open_ai_responses",
        WireFormat::OpenAiChatCompletions => "open_ai_chat_completions",
        WireFormat::AnthropicMessages => "anthropic_messages",
        WireFormat::GeminiGenerateContent => "gemini_generate_content",
        WireFormat::OpenAiEmbeddings => "open_ai_embeddings",
        WireFormat::OpenAiImageGenerations => "open_ai_image_generations",
        WireFormat::OpenAiAudioTranscriptions => "open_ai_audio_transcriptions",
        WireFormat::OpenAiAudioSpeech => "open_ai_audio_speech",
        WireFormat::OpenAiRerank => "open_ai_rerank",
    }
}

pub(super) fn dedupe_loss_reasons(mut reasons: Vec<String>) -> Vec<String> {
    reasons.sort();
    reasons.dedup();
    reasons
}
