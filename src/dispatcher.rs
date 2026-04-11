//! HTTP execution layer. Stateless — key injected per-request via auth scheme.

use std::pin::Pin;
use std::time::Duration;

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, StatusCode};

use crate::error::{ApiError, ProviderError};
use crate::key::lease::KeyLease;
use crate::protocol::{
    emit_request_with_mode, parse_error, parse_response, parse_stream_event, take_sse_frames,
    AuthScheme, ProtocolError, ProviderEndpoint, ProviderProtocol,
};
use crate::types::{LlmRequest, LlmResponse, LlmStreamEvent};

pub(crate) type EventStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, ApiError>> + Send>>;

/// Stateless HTTP executor.
pub(crate) struct Dispatcher {
    client: Client,
    provider_endpoint: ProviderEndpoint,
}

impl Dispatcher {
    pub(crate) fn new(provider_endpoint: ProviderEndpoint, timeout: Duration) -> Self {
        Self {
            client: Client::builder()
                .timeout(timeout)
                .build()
                .expect("failed to build reqwest client"),
            provider_endpoint,
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
                    if let Some(event) =
                        parse_stream_event(protocol, &frame).map_err(protocol_to_api)?
                    {
                        yield event;
                    }
                }
            }

            let tail = buffer.trim();
            if !tail.is_empty() {
                let frame = crate::protocol::ProviderStreamFrame {
                    event: None,
                    data: tail.to_string(),
                };
                if let Some(event) =
                    parse_stream_event(protocol, &frame).map_err(protocol_to_api)?
                {
                    yield event;
                }
            }
        };

        Ok(Box::pin(stream))
    }

    pub(crate) fn protocol(&self) -> ProviderProtocol {
        self.provider_endpoint.wire_protocol()
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

fn protocol_to_api(error: ProtocolError) -> ApiError {
    ApiError::Protocol(error.to_string())
}
