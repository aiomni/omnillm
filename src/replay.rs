//! Replay fixture helpers with default sanitization.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::api::{
    MultipartField, MultipartValue, RequestBody, ResponseBody, TransportRequest, TransportResponse,
    WireFormat,
};

/// A sanitized request/response exchange suitable for fixtures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFixture {
    pub wire_format: WireFormat,
    pub request: TransportRequest,
    pub response: TransportResponse,
}

impl ReplayFixture {
    pub fn sanitized(&self) -> Self {
        Self {
            wire_format: self.wire_format,
            request: sanitize_transport_request(&self.request),
            response: sanitize_transport_response(&self.response),
        }
    }
}

pub fn sanitize_transport_request(request: &TransportRequest) -> TransportRequest {
    TransportRequest {
        method: request.method,
        path: sanitize_path(&request.path),
        headers: sanitize_headers(&request.headers),
        accept: request.accept.clone(),
        body: sanitize_request_body(&request.body),
    }
}

pub fn sanitize_transport_response(response: &TransportResponse) -> TransportResponse {
    TransportResponse {
        status: response.status,
        headers: sanitize_headers(&response.headers),
        content_type: response.content_type.clone(),
        body: sanitize_response_body(&response.body),
    }
}

fn sanitize_request_body(body: &RequestBody) -> RequestBody {
    match body {
        RequestBody::Json { value } => RequestBody::Json {
            value: sanitize_json_value(value),
        },
        RequestBody::Multipart { fields } => RequestBody::Multipart {
            fields: fields.iter().map(sanitize_multipart_field).collect(),
        },
        RequestBody::Text { text } => RequestBody::Text {
            text: sanitize_text_field("body", text),
        },
        RequestBody::Binary { media_type, .. } => RequestBody::Binary {
            data_base64: redacted_blob("binary"),
            media_type: media_type.clone(),
        },
    }
}

fn sanitize_response_body(body: &ResponseBody) -> ResponseBody {
    match body {
        ResponseBody::Json { value } => ResponseBody::Json {
            value: sanitize_json_value(value),
        },
        ResponseBody::Text { text } => ResponseBody::Text { text: text.clone() },
        ResponseBody::Binary { media_type, .. } => ResponseBody::Binary {
            data_base64: redacted_blob("binary"),
            media_type: media_type.clone(),
        },
    }
}

fn sanitize_multipart_field(field: &MultipartField) -> MultipartField {
    MultipartField {
        name: field.name.clone(),
        value: match &field.value {
            MultipartValue::Text { value } => MultipartValue::Text {
                value: sanitize_text_field(&field.name, value),
            },
            MultipartValue::File {
                filename,
                media_type,
                ..
            } => MultipartValue::File {
                filename: filename.clone(),
                data_base64: redacted_blob("file"),
                media_type: media_type.clone(),
            },
        },
    }
}

pub fn sanitize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        if is_blob_key(key) {
                            Value::String(redacted_blob(key))
                        } else if is_sensitive_key(key) {
                            Value::String(redacted_value(key))
                        } else {
                            sanitize_json_value(value)
                        },
                    )
                })
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(sanitize_json_value).collect()),
        _ => value.clone(),
    }
}

fn sanitize_headers(
    headers: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                if is_sensitive_key(name) {
                    redacted_value(name)
                } else {
                    sanitize_text_field(name, value)
                },
            )
        })
        .collect()
}

fn sanitize_path(path: &str) -> String {
    let Some((base, query)) = path.split_once('?') else {
        return path.to_string();
    };

    let sanitized = query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            if is_sensitive_key(key) {
                format!("{key}={}", redacted_value(key))
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join("&");

    format!("{base}?{sanitized}")
}

fn sanitize_text_field(name: &str, value: &str) -> String {
    if is_sensitive_key(name) {
        redacted_value(name)
    } else {
        value.to_string()
    }
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "x-api-key"
            | "api-key"
            | "x-goog-api-key"
            | "cookie"
            | "set-cookie"
            | "api_key"
            | "apikey"
            | "access_token"
            | "token"
            | "secret"
            | "password"
            | "key"
            | "ak"
    )
}

fn is_blob_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "data_base64" | "image_base64" | "b64_json" | "audio_base64"
    )
}

fn redacted_value(label: &str) -> String {
    format!("<redacted:{label}>")
}

fn redacted_blob(label: &str) -> String {
    format!("<redacted:{label}_blob>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{HttpMethod, RequestBody, ResponseBody};

    #[test]
    fn sanitize_request_redacts_headers_query_and_nested_json() {
        let mut headers = std::collections::BTreeMap::new();
        headers.insert("Authorization".into(), "Bearer secret".into());

        let request = TransportRequest {
            method: HttpMethod::Post,
            path: "/responses?ak=123&foo=bar".into(),
            headers,
            accept: None,
            body: RequestBody::Json {
                value: serde_json::json!({
                    "api_key": "secret",
                    "nested": {
                        "token": "secret",
                        "keep": "ok"
                    },
                    "data_base64": "AAAA"
                }),
            },
        };

        let sanitized = sanitize_transport_request(&request);

        assert_eq!(
            sanitized.headers.get("Authorization").map(String::as_str),
            Some("<redacted:Authorization>")
        );
        assert_eq!(sanitized.path, "/responses?ak=<redacted:ak>&foo=bar");

        let RequestBody::Json { value } = sanitized.body else {
            panic!("expected json body");
        };
        assert_eq!(value["api_key"], "<redacted:api_key>");
        assert_eq!(value["nested"]["token"], "<redacted:token>");
        assert_eq!(value["nested"]["keep"], "ok");
        assert_eq!(value["data_base64"], "<redacted:data_base64_blob>");
    }

    #[test]
    fn sanitize_replay_fixture_redacts_binary_payloads() {
        let fixture = ReplayFixture {
            wire_format: WireFormat::OpenAiAudioSpeech,
            request: TransportRequest {
                method: HttpMethod::Post,
                path: "/audio/speech".into(),
                headers: Default::default(),
                accept: Some("audio/mpeg".into()),
                body: RequestBody::Json {
                    value: serde_json::json!({
                        "model": "tts-1",
                        "input": "hello"
                    }),
                },
            },
            response: TransportResponse {
                status: 200,
                headers: Default::default(),
                content_type: Some("audio/mpeg".into()),
                body: ResponseBody::Binary {
                    data_base64: "ZmFrZQ==".into(),
                    media_type: Some("audio/mpeg".into()),
                },
            },
        };

        let sanitized = fixture.sanitized();
        let ResponseBody::Binary { data_base64, .. } = sanitized.response.body else {
            panic!("expected binary response");
        };
        assert_eq!(data_base64, "<redacted:binary_blob>");
    }
}
