//! HTTP execution layer. Stateless — key injected per-request via auth scheme.

use std::pin::Pin;
use std::time::Duration;

use async_stream::try_stream;
use base64::prelude::{Engine as _, BASE64_STANDARD};
use futures_util::{Stream, StreamExt};
use reqwest::{Client, Method, StatusCode};

use crate::api::{HttpMethod, MultipartValue, RequestBody, ResponseBody};
use crate::error::{ApiError, ProviderError};
use crate::key::lease::KeyLease;
use crate::primitive::{
    extract_usage, primitive_error_from_body, PrimitiveProviderEndpoint, PrimitiveRequest,
    PrimitiveResponse, PrimitiveStreamEvent, PrimitiveStreamMode,
};
use crate::protocol::{
    emit_request_with_mode, parse_error, parse_response, parse_stream_events, take_sse_frames,
    AuthScheme, ProtocolError, ProviderEndpoint, ProviderProtocol, ProviderStreamFrame,
};
use crate::types::{LlmRequest, LlmResponse, LlmStreamEvent};

pub(crate) type EventStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, ApiError>> + Send>>;
pub(crate) type PrimitiveEventStream =
    Pin<Box<dyn Stream<Item = Result<PrimitiveStreamEvent, ApiError>> + Send>>;

/// Stateless HTTP executor.
pub(crate) struct Dispatcher {
    client: Client,
    provider_endpoint: ProviderEndpoint,
    primitive_endpoint: PrimitiveProviderEndpoint,
}

impl Dispatcher {
    pub(crate) fn new(provider_endpoint: ProviderEndpoint, timeout: Duration) -> Self {
        let primitive_endpoint = PrimitiveProviderEndpoint::from(&provider_endpoint);
        Self::new_with_primitive_endpoint(provider_endpoint, primitive_endpoint, timeout)
    }

    pub(crate) fn new_with_primitive_endpoint(
        provider_endpoint: ProviderEndpoint,
        primitive_endpoint: PrimitiveProviderEndpoint,
        timeout: Duration,
    ) -> Self {
        Self {
            client: Client::builder()
                .timeout(timeout)
                .build()
                .expect("failed to build reqwest client"),
            provider_endpoint,
            primitive_endpoint,
        }
    }

    pub(crate) async fn call(
        &self,
        lease: &KeyLease,
        req: &LlmRequest,
    ) -> Result<LlmResponse, ApiError> {
        let url = self.provider_endpoint.request_url(&req.model, false);
        let protocol = self.provider_endpoint.wire_protocol();
        let body = emit_request_with_mode(protocol, req, false).map_err(protocol_to_api)?;
        let response = self
            .request_builder(&url, &lease.inner.key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|error| network_to_api(protocol, error))?;

        let status = response.status();
        if status.is_success() {
            let text = response
                .text()
                .await
                .map_err(|error| network_to_api(protocol, error))?;
            parse_response(protocol, &text).map_err(protocol_to_api)
        } else {
            Err(self.classify_error(status, response).await)
        }
    }

    pub(crate) async fn stream(
        &self,
        lease: &KeyLease,
        req: &LlmRequest,
    ) -> Result<EventStream, ApiError> {
        let url = self.provider_endpoint.request_url(&req.model, true);
        let protocol = self.provider_endpoint.wire_protocol();
        let body = emit_request_with_mode(protocol, req, true).map_err(protocol_to_api)?;
        let response = self
            .request_builder(&url, &lease.inner.key)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .body(body)
            .send()
            .await
            .map_err(|error| network_to_api(protocol, error))?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.classify_error(status, response).await);
        }

        let stream = try_stream! {
            let mut buffer = String::new();
            let mut body_stream = response.bytes_stream();

            while let Some(chunk) = body_stream.next().await {
                let chunk = chunk.map_err(|error| network_to_api(protocol, error))?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                for frame in take_sse_frames(&mut buffer) {
                    for event in parse_stream_events(protocol, &frame).map_err(protocol_to_api)? {
                        yield event;
                    }
                }
            }

            let tail = buffer.trim();
            if !tail.is_empty() {
                let frame = ProviderStreamFrame {
                    event: None,
                    data: tail.to_string(),
                };
                for event in parse_stream_events(protocol, &frame).map_err(protocol_to_api)? {
                    yield event;
                }
            }
        };

        Ok(Box::pin(stream))
    }

    pub(crate) async fn primitive_call(
        &self,
        lease: &KeyLease,
        req: &PrimitiveRequest,
    ) -> Result<PrimitiveResponse, ApiError> {
        if req.stream != PrimitiveStreamMode::None {
            return Err(ApiError::Protocol(
                "primitive_call only supports non-stream primitive requests".into(),
            ));
        }

        let url = self
            .primitive_endpoint
            .request_url(req)
            .map_err(ApiError::Protocol)?;
        let response = self
            .primitive_request_builder(req, &url, &lease.inner.key)?
            .send()
            .await
            .map_err(|error| primitive_network_to_api(req, error))?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.classify_primitive_error(req, status, response).await);
        }

        self.primitive_response_from_reqwest(req, response).await
    }

    pub(crate) async fn primitive_stream(
        &self,
        lease: &KeyLease,
        req: &PrimitiveRequest,
    ) -> Result<PrimitiveEventStream, ApiError> {
        if req.stream != PrimitiveStreamMode::Sse {
            return Err(ApiError::Protocol(
                "primitive_stream currently supports SSE stream mode only".into(),
            ));
        }

        let url = self
            .primitive_endpoint
            .request_url(req)
            .map_err(ApiError::Protocol)?;
        let response = self
            .primitive_request_builder(req, &url, &lease.inner.key)?
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|error| primitive_network_to_api(req, error))?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.classify_primitive_error(req, status, response).await);
        }

        let wire_format = req.wire_format;
        let request_for_errors = req.clone();
        let stream = try_stream! {
            let mut buffer = String::new();
            let mut body_stream = response.bytes_stream();
            let mut latest_usage = None;

            while let Some(chunk) = body_stream.next().await {
                let chunk = chunk.map_err(|error| primitive_network_to_api(&request_for_errors, error))?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                for frame in take_sse_frames(&mut buffer) {
                    let event = PrimitiveStreamEvent::SseFrame {
                        event: frame.event.clone(),
                        data: frame.data.clone(),
                    };
                    if let Some(usage) = primitive_usage_from_sse_data(wire_format, &frame.data) {
                        latest_usage = Some(usage.clone());
                        yield event;
                        yield PrimitiveStreamEvent::Usage { usage };
                    } else {
                        yield event;
                    }
                }
            }

            let tail = buffer.trim();
            if !tail.is_empty() {
                let frame = PrimitiveStreamEvent::SseFrame {
                    event: None,
                    data: tail.to_string(),
                };
                if let Some(usage) = primitive_usage_from_sse_data(wire_format, tail) {
                    latest_usage = Some(usage.clone());
                    yield frame;
                    yield PrimitiveStreamEvent::Usage { usage };
                } else {
                    yield frame;
                }
            }

            yield PrimitiveStreamEvent::Completed { usage: latest_usage };
        };

        Ok(Box::pin(stream))
    }

    pub(crate) fn protocol(&self) -> ProviderProtocol {
        self.provider_endpoint.wire_protocol()
    }

    pub(crate) fn primitive_endpoint(&self) -> &PrimitiveProviderEndpoint {
        &self.primitive_endpoint
    }

    fn request_builder(&self, url: &str, api_key: &str) -> reqwest::RequestBuilder {
        let mut builder = self.client.post(url);
        for (name, value) in &self.provider_endpoint.default_headers {
            builder = builder.header(name, value);
        }

        match self.provider_endpoint.auth_scheme() {
            AuthScheme::Bearer => builder.bearer_auth(api_key),
            AuthScheme::Header { name } => builder.header(name, api_key),
            AuthScheme::Query { name } => builder.query(&[(name, api_key.to_string())]),
        }
    }

    fn primitive_request_builder(
        &self,
        request: &PrimitiveRequest,
        url: &str,
        api_key: &str,
    ) -> Result<reqwest::RequestBuilder, ApiError> {
        let mut builder = self.client.request(method(request.method), url);
        for (name, value) in &self.primitive_endpoint.default_headers {
            builder = builder.header(name, value);
        }
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(accept) = &request.accept {
            builder = builder.header("Accept", accept);
        }
        if !request.query.is_empty() {
            builder = builder.query(&request.query);
        }

        builder = match self.primitive_endpoint.auth_scheme() {
            AuthScheme::Bearer => builder.bearer_auth(api_key),
            AuthScheme::Header { name } => builder.header(name, api_key),
            AuthScheme::Query { name } => builder.query(&[(name, api_key.to_string())]),
        };

        match &request.body {
            RequestBody::Json { value } => Ok(builder.json(value)),
            RequestBody::Text { text } => Ok(builder
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(text.clone())),
            RequestBody::Binary {
                data_base64,
                media_type,
            } => {
                let bytes = BASE64_STANDARD
                    .decode(data_base64)
                    .map_err(|error| ApiError::Protocol(error.to_string()))?;
                let builder = if let Some(media_type) = media_type {
                    builder.header("Content-Type", media_type)
                } else {
                    builder
                };
                Ok(builder.body(bytes))
            }
            RequestBody::Multipart { fields } => {
                let mut form = reqwest::multipart::Form::new();
                for field in fields {
                    match &field.value {
                        MultipartValue::Text { value } => {
                            form = form.text(field.name.clone(), value.clone());
                        }
                        MultipartValue::File {
                            filename,
                            data_base64,
                            media_type,
                        } => {
                            let bytes = BASE64_STANDARD
                                .decode(data_base64)
                                .map_err(|error| ApiError::Protocol(error.to_string()))?;
                            let mut part =
                                reqwest::multipart::Part::bytes(bytes).file_name(filename.clone());
                            if let Some(media_type) = media_type {
                                part = part
                                    .mime_str(media_type)
                                    .map_err(|error| ApiError::Protocol(error.to_string()))?;
                            }
                            form = form.part(field.name.clone(), part);
                        }
                    }
                }
                Ok(builder.multipart(form))
            }
        }
    }

    async fn primitive_response_from_reqwest(
        &self,
        request: &PrimitiveRequest,
        response: reqwest::Response,
    ) -> Result<PrimitiveResponse, ApiError> {
        let status = response.status().as_u16();
        let headers = response_headers(response.headers());
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let bytes = response
            .bytes()
            .await
            .map_err(|error| primitive_network_to_api(request, error))?;
        let body = response_body(content_type.as_deref(), &bytes);
        let usage = extract_usage(request.wire_format, &body);

        Ok(PrimitiveResponse {
            provider: request.provider,
            endpoint: request.endpoint,
            wire_format: request.wire_format,
            status,
            headers,
            content_type,
            body,
            usage,
            metadata: Default::default(),
        })
    }

    async fn classify_error(&self, status: StatusCode, response: reqwest::Response) -> ApiError {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(60));
        let raw_body = response.text().await.unwrap_or_default();

        match status.as_u16() {
            401 | 403 => ApiError::Unauthorized,
            429 => ApiError::RateLimited { retry_after },
            _ => ApiError::Provider(
                parse_error(
                    self.provider_endpoint.wire_protocol(),
                    Some(status.as_u16()),
                    &raw_body,
                )
                .unwrap_or_else(|_| ProviderError {
                    protocol: self.provider_endpoint.wire_protocol(),
                    status: Some(status.as_u16()),
                    code: None,
                    message: raw_body,
                    retry_after: None,
                    raw_body: None,
                    vendor_extensions: Default::default(),
                }),
            ),
        }
    }

    async fn classify_primitive_error(
        &self,
        request: &PrimitiveRequest,
        status: StatusCode,
        response: reqwest::Response,
    ) -> ApiError {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(60));
        let raw_body = response.text().await.unwrap_or_default();

        match status.as_u16() {
            401 | 403 => ApiError::Unauthorized,
            429 => ApiError::RateLimited { retry_after },
            _ => ApiError::PrimitiveProvider(primitive_error_from_body(
                request.provider,
                request.wire_format,
                Some(status.as_u16()),
                Some(retry_after),
                raw_body,
            )),
        }
    }
}

fn method(method: HttpMethod) -> Method {
    match method {
        HttpMethod::Get => Method::GET,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
    }
}

fn response_headers(
    headers: &reqwest::header::HeaderMap,
) -> std::collections::BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect()
}

fn response_body(content_type: Option<&str>, bytes: &[u8]) -> ResponseBody {
    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type.contains("json") {
        if let Ok(value) = serde_json::from_slice(bytes) {
            return ResponseBody::Json { value };
        }
    }
    if content_type.starts_with("text/") || content_type.contains("event-stream") {
        return ResponseBody::Text {
            text: String::from_utf8_lossy(bytes).into_owned(),
        };
    }
    if let Ok(value) = serde_json::from_slice(bytes) {
        return ResponseBody::Json { value };
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return ResponseBody::Text {
            text: text.to_string(),
        };
    }
    ResponseBody::Binary {
        data_base64: BASE64_STANDARD.encode(bytes),
        media_type: if content_type.is_empty() {
            None
        } else {
            Some(content_type)
        },
    }
}

fn primitive_usage_from_sse_data(
    wire_format: crate::primitive::ProviderPrimitiveWireFormat,
    data: &str,
) -> Option<crate::primitive::PrimitiveUsageTelemetry> {
    if data.trim() == "[DONE]" {
        return None;
    }
    let value = serde_json::from_str::<serde_json::Value>(data).ok()?;
    let body = ResponseBody::Json { value };
    extract_usage(wire_format, &body)
}

fn network_to_api(protocol: ProviderProtocol, error: reqwest::Error) -> ApiError {
    ApiError::Provider(ProviderError {
        protocol,
        status: None,
        code: Some("network_error".into()),
        message: error.to_string(),
        retry_after: None,
        raw_body: None,
        vendor_extensions: Default::default(),
    })
}

fn primitive_network_to_api(request: &PrimitiveRequest, error: reqwest::Error) -> ApiError {
    ApiError::PrimitiveProvider(primitive_error_from_body(
        request.provider,
        request.wire_format,
        None,
        None,
        error.to_string(),
    ))
}

fn protocol_to_api(error: ProtocolError) -> ApiError {
    ApiError::Protocol(error.to_string())
}
