//! Multi-endpoint canonical <-> wire conversion helpers.

use serde_json::Value;
use thiserror::Error;

use crate::api::{
    ApiRequest, ApiResponse, AudioSpeechResponse, ConversionReport, EndpointKind, HttpMethod,
    RequestBody, ResponseBody, TransportRequest, TransportResponse, WireFormat,
};
use crate::protocol::{
    emit_request, emit_request_with_mode, emit_response, parse_request, parse_response,
    ProtocolError,
};
use crate::types::VendorExtensions;

mod audio;
mod common;
mod embeddings;
mod generation;
mod images;
mod rerank;

use audio::{
    emit_openai_audio_speech_request, emit_openai_audio_transcription_response,
    emit_openai_audio_transcription_transport, parse_openai_audio_speech_request,
    parse_openai_audio_transcription_response,
};
use common::{audio_media_type_for_format, json_content_headers, json_response_body};
use embeddings::{
    emit_openai_embeddings_request, emit_openai_embeddings_response,
    parse_openai_embeddings_request, parse_openai_embeddings_response,
};
use generation::{
    ensure_matching_endpoint, generation_protocol, generation_request_report,
    generation_response_report, generation_string_report, generation_transport_report,
    sanitize_generation_request, wire_path,
};
use images::{
    emit_openai_image_generation_request, emit_openai_image_generation_response,
    parse_openai_image_generation_request, parse_openai_image_generation_response,
};
use rerank::{
    emit_openai_rerank_request, emit_openai_rerank_response, parse_openai_rerank_request,
    parse_openai_rerank_response,
};

/// Error returned by the multi-endpoint conversion layer.
#[derive(Debug, Error)]
pub enum ApiProtocolError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Generation(#[from] ProtocolError),
    #[error("endpoint mismatch: expected {expected:?}, got {actual:?}")]
    EndpointMismatch {
        expected: EndpointKind,
        actual: EndpointKind,
    },
    #[error("unsupported wire format {wire_format:?} for endpoint {endpoint:?}")]
    UnsupportedWireFormat {
        endpoint: EndpointKind,
        wire_format: WireFormat,
    },
    #[error("multipart or binary transport required for wire format {0:?}")]
    TransportRequired(WireFormat),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid shape: {0}")]
    InvalidShape(String),
    #[error("unsupported feature for wire format {wire_format:?}: {message}")]
    UnsupportedFeature {
        wire_format: WireFormat,
        message: String,
    },
}

pub fn parse_api_request(
    wire_format: WireFormat,
    raw_json: &str,
) -> Result<ConversionReport<ApiRequest>, ApiProtocolError> {
    match wire_format {
        WireFormat::OpenAiResponses
        | WireFormat::OpenAiChatCompletions
        | WireFormat::AnthropicMessages
        | WireFormat::GeminiGenerateContent => {
            let protocol = generation_protocol(wire_format)?;
            let request = parse_request(protocol, raw_json)?;
            Ok(generation_request_report(
                wire_format,
                ApiRequest::Responses(request),
                Vec::new(),
            ))
        }
        WireFormat::OpenAiEmbeddings => {
            let body: Value = serde_json::from_str(raw_json)?;
            let request = parse_openai_embeddings_request(&body)?;
            Ok(ConversionReport::native(
                ApiRequest::Embeddings(request),
                EndpointKind::Embeddings,
                wire_format,
            ))
        }
        WireFormat::OpenAiImageGenerations => {
            let body: Value = serde_json::from_str(raw_json)?;
            let request = parse_openai_image_generation_request(&body)?;
            Ok(ConversionReport::native(
                ApiRequest::ImageGenerations(request),
                EndpointKind::ImageGenerations,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioTranscriptions => {
            Err(ApiProtocolError::TransportRequired(wire_format))
        }
        WireFormat::OpenAiAudioSpeech => {
            let body: Value = serde_json::from_str(raw_json)?;
            let request = parse_openai_audio_speech_request(&body)?;
            Ok(ConversionReport::native(
                ApiRequest::AudioSpeech(request),
                EndpointKind::AudioSpeech,
                wire_format,
            ))
        }
        WireFormat::OpenAiRerank => {
            let body: Value = serde_json::from_str(raw_json)?;
            let request = parse_openai_rerank_request(&body)?;
            Ok(ConversionReport::native(
                ApiRequest::Rerank(request),
                EndpointKind::Rerank,
                wire_format,
            ))
        }
    }
}

pub fn emit_api_request(
    wire_format: WireFormat,
    request: &ApiRequest,
) -> Result<ConversionReport<String>, ApiProtocolError> {
    ensure_matching_endpoint(wire_format, request.canonical_endpoint_kind())?;

    match wire_format {
        WireFormat::OpenAiResponses
        | WireFormat::OpenAiChatCompletions
        | WireFormat::AnthropicMessages
        | WireFormat::GeminiGenerateContent => {
            let protocol = generation_protocol(wire_format)?;
            let request =
                request
                    .clone()
                    .try_into()
                    .map_err(|_| ApiProtocolError::EndpointMismatch {
                        expected: EndpointKind::Responses,
                        actual: request.canonical_endpoint_kind(),
                    })?;
            let (sanitized, loss_reasons) = sanitize_generation_request(wire_format, &request)?;
            let raw = emit_request(protocol, &sanitized)?;
            Ok(generation_string_report(wire_format, raw, loss_reasons))
        }
        WireFormat::OpenAiEmbeddings => {
            let ApiRequest::Embeddings(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_embeddings_request(request)?)?,
                EndpointKind::Embeddings,
                wire_format,
            ))
        }
        WireFormat::OpenAiImageGenerations => {
            let ApiRequest::ImageGenerations(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_image_generation_request(request))?,
                EndpointKind::ImageGenerations,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioTranscriptions => {
            Err(ApiProtocolError::TransportRequired(wire_format))
        }
        WireFormat::OpenAiAudioSpeech => {
            let ApiRequest::AudioSpeech(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_audio_speech_request(request))?,
                EndpointKind::AudioSpeech,
                wire_format,
            ))
        }
        WireFormat::OpenAiRerank => {
            let ApiRequest::Rerank(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_rerank_request(request))?,
                EndpointKind::Rerank,
                wire_format,
            ))
        }
    }
}

pub fn transcode_api_request(
    from: WireFormat,
    to: WireFormat,
    raw_json: &str,
) -> Result<ConversionReport<String>, ApiProtocolError> {
    let parsed = parse_api_request(from, raw_json)?;
    let emitted = emit_api_request(to, &parsed.value)?;

    let mut loss_reasons = parsed.loss_reasons;
    loss_reasons.extend(emitted.loss_reasons);

    Ok(ConversionReport {
        value: emitted.value,
        canonical_endpoint: emitted.canonical_endpoint,
        wire_format: emitted.wire_format,
        bridged: from != to || emitted.bridged || parsed.bridged,
        lossy: !loss_reasons.is_empty(),
        loss_reasons,
    })
}

pub fn parse_api_response(
    wire_format: WireFormat,
    raw_json: &str,
) -> Result<ConversionReport<ApiResponse>, ApiProtocolError> {
    match wire_format {
        WireFormat::OpenAiResponses
        | WireFormat::OpenAiChatCompletions
        | WireFormat::AnthropicMessages
        | WireFormat::GeminiGenerateContent => {
            let protocol = generation_protocol(wire_format)?;
            let response = parse_response(protocol, raw_json)?;
            Ok(generation_response_report(
                wire_format,
                ApiResponse::Responses(response),
                Vec::new(),
            ))
        }
        WireFormat::OpenAiEmbeddings => {
            let body: Value = serde_json::from_str(raw_json)?;
            let response = parse_openai_embeddings_response(&body)?;
            Ok(ConversionReport::native(
                ApiResponse::Embeddings(response),
                EndpointKind::Embeddings,
                wire_format,
            ))
        }
        WireFormat::OpenAiImageGenerations => {
            let body: Value = serde_json::from_str(raw_json)?;
            let response = parse_openai_image_generation_response(&body)?;
            Ok(ConversionReport::native(
                ApiResponse::ImageGenerations(response),
                EndpointKind::ImageGenerations,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioTranscriptions => {
            let body: Value = serde_json::from_str(raw_json)?;
            let response = parse_openai_audio_transcription_response(&body)?;
            Ok(ConversionReport::native(
                ApiResponse::AudioTranscriptions(response),
                EndpointKind::AudioTranscriptions,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioSpeech => Err(ApiProtocolError::TransportRequired(wire_format)),
        WireFormat::OpenAiRerank => {
            let body: Value = serde_json::from_str(raw_json)?;
            let response = parse_openai_rerank_response(&body)?;
            Ok(ConversionReport::native(
                ApiResponse::Rerank(response),
                EndpointKind::Rerank,
                wire_format,
            ))
        }
    }
}

pub fn emit_api_response(
    wire_format: WireFormat,
    response: &ApiResponse,
) -> Result<ConversionReport<String>, ApiProtocolError> {
    ensure_matching_endpoint(wire_format, response.canonical_endpoint_kind())?;

    match wire_format {
        WireFormat::OpenAiResponses
        | WireFormat::OpenAiChatCompletions
        | WireFormat::AnthropicMessages
        | WireFormat::GeminiGenerateContent => {
            let protocol = generation_protocol(wire_format)?;
            let response =
                response
                    .clone()
                    .try_into()
                    .map_err(|_| ApiProtocolError::EndpointMismatch {
                        expected: EndpointKind::Responses,
                        actual: response.canonical_endpoint_kind(),
                    })?;
            let raw = emit_response(protocol, &response)?;
            Ok(generation_string_report(wire_format, raw, Vec::new()))
        }
        WireFormat::OpenAiEmbeddings => {
            let ApiResponse::Embeddings(response) = response else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_embeddings_response(response))?,
                EndpointKind::Embeddings,
                wire_format,
            ))
        }
        WireFormat::OpenAiImageGenerations => {
            let ApiResponse::ImageGenerations(response) = response else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_image_generation_response(response))?,
                EndpointKind::ImageGenerations,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioTranscriptions => {
            let ApiResponse::AudioTranscriptions(response) = response else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_audio_transcription_response(response))?,
                EndpointKind::AudioTranscriptions,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioSpeech => Err(ApiProtocolError::TransportRequired(wire_format)),
        WireFormat::OpenAiRerank => {
            let ApiResponse::Rerank(response) = response else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                serde_json::to_string(&emit_openai_rerank_response(response))?,
                EndpointKind::Rerank,
                wire_format,
            ))
        }
    }
}

pub fn transcode_api_response(
    from: WireFormat,
    to: WireFormat,
    raw_json: &str,
) -> Result<ConversionReport<String>, ApiProtocolError> {
    let parsed = parse_api_response(from, raw_json)?;
    let emitted = emit_api_response(to, &parsed.value)?;

    let mut loss_reasons = parsed.loss_reasons;
    loss_reasons.extend(emitted.loss_reasons);

    Ok(ConversionReport {
        value: emitted.value,
        canonical_endpoint: emitted.canonical_endpoint,
        wire_format: emitted.wire_format,
        bridged: from != to || emitted.bridged || parsed.bridged,
        lossy: !loss_reasons.is_empty(),
        loss_reasons,
    })
}

pub fn emit_transport_request(
    wire_format: WireFormat,
    request: &ApiRequest,
) -> Result<ConversionReport<TransportRequest>, ApiProtocolError> {
    ensure_matching_endpoint(wire_format, request.canonical_endpoint_kind())?;

    match wire_format {
        WireFormat::OpenAiResponses
        | WireFormat::OpenAiChatCompletions
        | WireFormat::AnthropicMessages
        | WireFormat::GeminiGenerateContent => {
            let protocol = generation_protocol(wire_format)?;
            let request =
                request
                    .clone()
                    .try_into()
                    .map_err(|_| ApiProtocolError::EndpointMismatch {
                        expected: EndpointKind::Responses,
                        actual: request.canonical_endpoint_kind(),
                    })?;
            let (sanitized, loss_reasons) = sanitize_generation_request(wire_format, &request)?;
            let body: Value =
                serde_json::from_str(&emit_request_with_mode(protocol, &sanitized, false)?)?;
            let transport = TransportRequest {
                method: HttpMethod::Post,
                path: wire_path(wire_format, &sanitized.model),
                headers: json_content_headers(),
                accept: None,
                body: RequestBody::Json { value: body },
            };

            Ok(generation_transport_report(
                wire_format,
                transport,
                loss_reasons,
            ))
        }
        WireFormat::OpenAiEmbeddings => {
            let ApiRequest::Embeddings(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                TransportRequest {
                    method: HttpMethod::Post,
                    path: wire_path(wire_format, &request.model),
                    headers: json_content_headers(),
                    accept: None,
                    body: RequestBody::Json {
                        value: emit_openai_embeddings_request(request)?,
                    },
                },
                EndpointKind::Embeddings,
                wire_format,
            ))
        }
        WireFormat::OpenAiImageGenerations => {
            let ApiRequest::ImageGenerations(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                TransportRequest {
                    method: HttpMethod::Post,
                    path: wire_path(wire_format, request.model.as_deref().unwrap_or("")),
                    headers: json_content_headers(),
                    accept: None,
                    body: RequestBody::Json {
                        value: emit_openai_image_generation_request(request),
                    },
                },
                EndpointKind::ImageGenerations,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioTranscriptions => {
            let ApiRequest::AudioTranscriptions(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                emit_openai_audio_transcription_transport(request)?,
                EndpointKind::AudioTranscriptions,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioSpeech => {
            let ApiRequest::AudioSpeech(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                TransportRequest {
                    method: HttpMethod::Post,
                    path: wire_path(wire_format, &request.model),
                    headers: json_content_headers(),
                    accept: Some(audio_media_type_for_format(
                        request.response_format.as_deref(),
                    )),
                    body: RequestBody::Json {
                        value: emit_openai_audio_speech_request(request),
                    },
                },
                EndpointKind::AudioSpeech,
                wire_format,
            ))
        }
        WireFormat::OpenAiRerank => {
            let ApiRequest::Rerank(request) = request else {
                unreachable!("checked by ensure_matching_endpoint");
            };
            Ok(ConversionReport::native(
                TransportRequest {
                    method: HttpMethod::Post,
                    path: wire_path(wire_format, &request.model),
                    headers: json_content_headers(),
                    accept: None,
                    body: RequestBody::Json {
                        value: emit_openai_rerank_request(request),
                    },
                },
                EndpointKind::Rerank,
                wire_format,
            ))
        }
    }
}

pub fn parse_transport_response(
    wire_format: WireFormat,
    response: &TransportResponse,
) -> Result<ConversionReport<ApiResponse>, ApiProtocolError> {
    match wire_format {
        WireFormat::OpenAiResponses
        | WireFormat::OpenAiChatCompletions
        | WireFormat::AnthropicMessages
        | WireFormat::GeminiGenerateContent => {
            let protocol = generation_protocol(wire_format)?;
            let body = json_response_body(response)?;
            let response = parse_response(protocol, &body.to_string())?;
            Ok(generation_response_report(
                wire_format,
                ApiResponse::Responses(response),
                Vec::new(),
            ))
        }
        WireFormat::OpenAiEmbeddings => {
            let body = json_response_body(response)?;
            Ok(ConversionReport::native(
                ApiResponse::Embeddings(parse_openai_embeddings_response(body)?),
                EndpointKind::Embeddings,
                wire_format,
            ))
        }
        WireFormat::OpenAiImageGenerations => {
            let body = json_response_body(response)?;
            Ok(ConversionReport::native(
                ApiResponse::ImageGenerations(parse_openai_image_generation_response(body)?),
                EndpointKind::ImageGenerations,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioTranscriptions => {
            let body = json_response_body(response)?;
            Ok(ConversionReport::native(
                ApiResponse::AudioTranscriptions(parse_openai_audio_transcription_response(body)?),
                EndpointKind::AudioTranscriptions,
                wire_format,
            ))
        }
        WireFormat::OpenAiAudioSpeech => match &response.body {
            ResponseBody::Binary {
                data_base64,
                media_type,
            } => Ok(ConversionReport::native(
                ApiResponse::AudioSpeech(AudioSpeechResponse {
                    data_base64: data_base64.clone(),
                    media_type: media_type.clone().or_else(|| response.content_type.clone()),
                    vendor_extensions: VendorExtensions::new(),
                }),
                EndpointKind::AudioSpeech,
                wire_format,
            )),
            _ => Err(ApiProtocolError::InvalidShape(
                "audio speech response body must be binary".into(),
            )),
        },
        WireFormat::OpenAiRerank => {
            let body = json_response_body(response)?;
            Ok(ConversionReport::native(
                ApiResponse::Rerank(parse_openai_rerank_response(body)?),
                EndpointKind::Rerank,
                wire_format,
            ))
        }
    }
}

#[cfg(test)]
mod tests;
