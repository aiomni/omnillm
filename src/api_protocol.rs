//! Multi-endpoint canonical <-> wire conversion helpers.

use serde_json::{json, Map, Value};
use thiserror::Error;

use crate::api::{
    ApiRequest, ApiResponse, AudioInput, AudioSegment, AudioSpeechRequest, AudioSpeechResponse,
    AudioTranscriptionRequest, AudioTranscriptionResponse, ConversionReport, EmbeddingInput,
    EmbeddingRequest, EmbeddingResponse, EmbeddingUsage, EmbeddingVector, EndpointKind,
    GeneratedImage, HttpMethod, ImageGenerationRequest, ImageGenerationResponse, MultipartField,
    MultipartValue, RequestBody, RerankDocument, RerankRequest, RerankResponse, RerankResult,
    RerankUsage, ResponseBody, TranscribedWord, TransportRequest, TransportResponse, WireFormat,
};
use crate::protocol::{
    emit_request, emit_request_with_mode, emit_response, parse_request, parse_response,
    ProtocolError, ProviderProtocol,
};
use crate::types::{BuiltinTool, LlmRequest, VendorExtensions};

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
            let (sanitized, loss_reasons) = sanitize_generation_request(wire_format, &request);
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
            let (sanitized, loss_reasons) = sanitize_generation_request(wire_format, &request);
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

fn generation_protocol(wire_format: WireFormat) -> Result<ProviderProtocol, ApiProtocolError> {
    ProviderProtocol::try_from(wire_format).map_err(|_| ApiProtocolError::UnsupportedWireFormat {
        endpoint: EndpointKind::Responses,
        wire_format,
    })
}

fn ensure_matching_endpoint(
    wire_format: WireFormat,
    endpoint: EndpointKind,
) -> Result<(), ApiProtocolError> {
    let expected = wire_format.canonical_endpoint_kind();
    if expected == endpoint {
        Ok(())
    } else {
        Err(ApiProtocolError::EndpointMismatch {
            expected,
            actual: endpoint,
        })
    }
}

fn generation_request_report(
    wire_format: WireFormat,
    value: ApiRequest,
    loss_reasons: Vec<String>,
) -> ConversionReport<ApiRequest> {
    if wire_format == WireFormat::OpenAiResponses {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

fn generation_response_report(
    wire_format: WireFormat,
    value: ApiResponse,
    loss_reasons: Vec<String>,
) -> ConversionReport<ApiResponse> {
    if wire_format == WireFormat::OpenAiResponses {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

fn generation_string_report(
    wire_format: WireFormat,
    value: String,
    loss_reasons: Vec<String>,
) -> ConversionReport<String> {
    if wire_format == WireFormat::OpenAiResponses {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

fn generation_transport_report(
    wire_format: WireFormat,
    value: TransportRequest,
    loss_reasons: Vec<String>,
) -> ConversionReport<TransportRequest> {
    if wire_format == WireFormat::OpenAiResponses {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

fn sanitize_generation_request(
    wire_format: WireFormat,
    request: &LlmRequest,
) -> (LlmRequest, Vec<String>) {
    let mut sanitized = request.clone();
    let mut loss_reasons = Vec::new();

    match wire_format {
        WireFormat::OpenAiResponses => {}
        WireFormat::OpenAiChatCompletions => {
            if !sanitized.capabilities.builtin_tools.is_empty() {
                sanitized.capabilities.builtin_tools.clear();
                loss_reasons.push(
                    "builtin tools are dropped when bridging to open_ai_chat_completions".into(),
                );
            }
            if sanitized.capabilities.reasoning.take().is_some() {
                loss_reasons.push(
                    "reasoning settings are dropped when bridging to open_ai_chat_completions"
                        .into(),
                );
            }
        }
        WireFormat::AnthropicMessages => {
            if !sanitized.capabilities.builtin_tools.is_empty() {
                sanitized.capabilities.builtin_tools.clear();
                loss_reasons
                    .push("builtin tools are dropped when bridging to anthropic_messages".into());
            }
            if sanitized.capabilities.structured_output.take().is_some() {
                loss_reasons.push(
                    "structured output is dropped when bridging to anthropic_messages".into(),
                );
            }
            if sanitized.capabilities.reasoning.take().is_some() {
                loss_reasons.push(
                    "reasoning settings are dropped when bridging to anthropic_messages".into(),
                );
            }
            if sanitized.generation.top_k.take().is_some() {
                loss_reasons.push("top_k is dropped when bridging to anthropic_messages".into());
            }
            if sanitized.generation.presence_penalty.take().is_some() {
                loss_reasons
                    .push("presence_penalty is dropped when bridging to anthropic_messages".into());
            }
            if sanitized.generation.frequency_penalty.take().is_some() {
                loss_reasons.push(
                    "frequency_penalty is dropped when bridging to anthropic_messages".into(),
                );
            }
            if sanitized.generation.seed.take().is_some() {
                loss_reasons.push("seed is dropped when bridging to anthropic_messages".into());
            }
        }
        WireFormat::GeminiGenerateContent => {
            let before = sanitized.capabilities.builtin_tools.len();
            sanitized
                .capabilities
                .builtin_tools
                .retain(|tool| matches!(tool, BuiltinTool::CodeExecution));
            if sanitized.capabilities.builtin_tools.len() != before {
                loss_reasons.push(
                    "only code_execution builtin tools are preserved for gemini_generate_content"
                        .into(),
                );
            }
            if sanitized.capabilities.reasoning.take().is_some() {
                loss_reasons.push(
                    "reasoning settings are dropped when bridging to gemini_generate_content"
                        .into(),
                );
            }
            if sanitized.generation.presence_penalty.take().is_some() {
                loss_reasons.push(
                    "presence_penalty is dropped when bridging to gemini_generate_content".into(),
                );
            }
            if sanitized.generation.frequency_penalty.take().is_some() {
                loss_reasons.push(
                    "frequency_penalty is dropped when bridging to gemini_generate_content".into(),
                );
            }
        }
        _ => {}
    }

    if wire_format != WireFormat::OpenAiResponses && !sanitized.metadata.is_empty() {
        sanitized.metadata.clear();
        loss_reasons.push(format!(
            "metadata is dropped when bridging to {}",
            wire_format_name(wire_format)
        ));
    }

    if wire_format != WireFormat::OpenAiResponses && request_has_vendor_extensions(request) {
        loss_reasons.push(format!(
            "vendor_extensions and raw_message fields are not emitted to {}",
            wire_format_name(wire_format)
        ));
    }

    (sanitized, dedupe_loss_reasons(loss_reasons))
}

fn request_has_vendor_extensions(request: &LlmRequest) -> bool {
    if !request.vendor_extensions.is_empty() || !request.capabilities.vendor_extensions.is_empty() {
        return true;
    }

    if request.normalized_input().iter().any(|item| match item {
        crate::types::RequestItem::Message { message } => {
            message.raw_message.is_some() || !message.vendor_extensions.is_empty()
        }
        crate::types::RequestItem::ToolResult { .. } => false,
    }) {
        return true;
    }

    request
        .capabilities
        .reasoning
        .as_ref()
        .is_some_and(|reasoning| !reasoning.vendor_extensions.is_empty())
        || request
            .capabilities
            .tools
            .iter()
            .any(|tool| !tool.vendor_extensions.is_empty())
}

fn wire_path(wire_format: WireFormat, model: &str) -> String {
    match wire_format {
        WireFormat::OpenAiResponses => "/responses".into(),
        WireFormat::OpenAiChatCompletions => "/chat/completions".into(),
        WireFormat::AnthropicMessages => "/messages".into(),
        WireFormat::GeminiGenerateContent => format!("/models/{model}:generateContent"),
        WireFormat::OpenAiEmbeddings => "/embeddings".into(),
        WireFormat::OpenAiImageGenerations => "/images/generations".into(),
        WireFormat::OpenAiAudioTranscriptions => "/audio/transcriptions".into(),
        WireFormat::OpenAiAudioSpeech => "/audio/speech".into(),
        WireFormat::OpenAiRerank => "/rerank".into(),
    }
}

fn emit_openai_embeddings_request(request: &EmbeddingRequest) -> Result<Value, ApiProtocolError> {
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

fn parse_openai_embeddings_request(body: &Value) -> Result<EmbeddingRequest, ApiProtocolError> {
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

fn emit_openai_embeddings_response(response: &EmbeddingResponse) -> Value {
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

fn parse_openai_embeddings_response(body: &Value) -> Result<EmbeddingResponse, ApiProtocolError> {
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

fn emit_openai_image_generation_request(request: &ImageGenerationRequest) -> Value {
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

fn parse_openai_image_generation_request(
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

fn emit_openai_image_generation_response(response: &ImageGenerationResponse) -> Value {
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

fn parse_openai_image_generation_response(
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

fn emit_openai_audio_transcription_transport(
    request: &AudioTranscriptionRequest,
) -> Result<TransportRequest, ApiProtocolError> {
    let mut fields = vec![MultipartField {
        name: "model".into(),
        value: MultipartValue::Text {
            value: request.model.clone(),
        },
    }];

    match &request.audio {
        AudioInput::File {
            filename,
            data_base64,
            media_type,
        } => fields.push(MultipartField {
            name: "file".into(),
            value: MultipartValue::File {
                filename: filename.clone(),
                data_base64: data_base64.clone(),
                media_type: media_type.clone(),
            },
        }),
        AudioInput::Url { .. } => {
            return Err(ApiProtocolError::UnsupportedFeature {
                wire_format: WireFormat::OpenAiAudioTranscriptions,
                message: "audio URL inputs are not supported by multipart transcription requests"
                    .into(),
            })
        }
    }

    if let Some(prompt) = &request.prompt {
        fields.push(MultipartField {
            name: "prompt".into(),
            value: MultipartValue::Text {
                value: prompt.clone(),
            },
        });
    }
    if let Some(response_format) = &request.response_format {
        fields.push(MultipartField {
            name: "response_format".into(),
            value: MultipartValue::Text {
                value: response_format.clone(),
            },
        });
    }
    if let Some(language) = &request.language {
        fields.push(MultipartField {
            name: "language".into(),
            value: MultipartValue::Text {
                value: language.clone(),
            },
        });
    }
    if let Some(temperature) = request.temperature {
        fields.push(MultipartField {
            name: "temperature".into(),
            value: MultipartValue::Text {
                value: temperature.to_string(),
            },
        });
    }
    for granularity in &request.timestamp_granularities {
        fields.push(MultipartField {
            name: "timestamp_granularities[]".into(),
            value: MultipartValue::Text {
                value: granularity.clone(),
            },
        });
    }

    Ok(TransportRequest {
        method: HttpMethod::Post,
        path: wire_path(WireFormat::OpenAiAudioTranscriptions, &request.model),
        headers: Default::default(),
        accept: Some("application/json".into()),
        body: RequestBody::Multipart { fields },
    })
}

fn emit_openai_audio_speech_request(request: &AudioSpeechRequest) -> Value {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert("input".into(), Value::String(request.input.clone()));
    map.insert("voice".into(), Value::String(request.voice.clone()));
    if let Some(response_format) = &request.response_format {
        map.insert(
            "response_format".into(),
            Value::String(response_format.clone()),
        );
    }
    if let Some(speed) = request.speed {
        map.insert("speed".into(), Value::from(speed));
    }
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);
    Value::Object(map)
}

fn parse_openai_audio_speech_request(body: &Value) -> Result<AudioSpeechRequest, ApiProtocolError> {
    Ok(AudioSpeechRequest {
        model: required_str(body, "model")?.to_string(),
        input: required_str(body, "input")?.to_string(),
        voice: required_str(body, "voice")?.to_string(),
        response_format: body
            .get("response_format")
            .and_then(Value::as_str)
            .map(str::to_owned),
        speed: body
            .get("speed")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        vendor_extensions: collect_vendor_extensions(
            body,
            &["model", "input", "voice", "response_format", "speed"],
        ),
    })
}

fn emit_openai_audio_transcription_response(response: &AudioTranscriptionResponse) -> Value {
    let mut map = Map::new();
    map.insert("text".into(), Value::String(response.text.clone()));
    if let Some(language) = &response.language {
        map.insert("language".into(), Value::String(language.clone()));
    }
    if let Some(duration) = response.duration_seconds {
        map.insert("duration".into(), Value::from(duration));
    }
    if !response.segments.is_empty() {
        map.insert(
            "segments".into(),
            Value::Array(
                response
                    .segments
                    .iter()
                    .map(|segment| {
                        let mut segment_map = Map::new();
                        if let Some(id) = segment.id {
                            segment_map.insert("id".into(), Value::from(id));
                        }
                        if let Some(start) = segment.start {
                            segment_map.insert("start".into(), Value::from(start));
                        }
                        if let Some(end) = segment.end {
                            segment_map.insert("end".into(), Value::from(end));
                        }
                        segment_map.insert("text".into(), Value::String(segment.text.clone()));
                        Value::Object(segment_map)
                    })
                    .collect(),
            ),
        );
    }
    if !response.words.is_empty() {
        map.insert(
            "words".into(),
            Value::Array(
                response
                    .words
                    .iter()
                    .map(|word| {
                        let mut word_map = Map::new();
                        word_map.insert("word".into(), Value::String(word.word.clone()));
                        if let Some(start) = word.start {
                            word_map.insert("start".into(), Value::from(start));
                        }
                        if let Some(end) = word.end {
                            word_map.insert("end".into(), Value::from(end));
                        }
                        Value::Object(word_map)
                    })
                    .collect(),
            ),
        );
    }
    extend_with_vendor_extensions(&mut map, &response.vendor_extensions);
    Value::Object(map)
}

fn parse_openai_audio_transcription_response(
    body: &Value,
) -> Result<AudioTranscriptionResponse, ApiProtocolError> {
    let segments = body
        .get("segments")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .map(|segment| AudioSegment {
                    id: segment
                        .get("id")
                        .and_then(Value::as_u64)
                        .map(|value| value as u32),
                    start: segment
                        .get("start")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                    end: segment
                        .get("end")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                    text: segment
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let words = body
        .get("words")
        .and_then(Value::as_array)
        .map(|words| {
            words
                .iter()
                .map(|word| TranscribedWord {
                    word: word
                        .get("word")
                        .or_else(|| word.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    start: word
                        .get("start")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                    end: word
                        .get("end")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(AudioTranscriptionResponse {
        text: required_str(body, "text")?.to_string(),
        language: body
            .get("language")
            .and_then(Value::as_str)
            .map(str::to_owned),
        duration_seconds: body
            .get("duration")
            .or_else(|| body.get("duration_seconds"))
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        segments,
        words,
        vendor_extensions: collect_vendor_extensions(
            body,
            &[
                "text",
                "language",
                "duration",
                "duration_seconds",
                "segments",
                "words",
            ],
        ),
    })
}

fn emit_openai_rerank_request(request: &RerankRequest) -> Value {
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

fn parse_openai_rerank_request(body: &Value) -> Result<RerankRequest, ApiProtocolError> {
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

fn emit_openai_rerank_response(response: &RerankResponse) -> Value {
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

fn parse_openai_rerank_response(body: &Value) -> Result<RerankResponse, ApiProtocolError> {
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

fn json_content_headers() -> std::collections::BTreeMap<String, String> {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    headers
}

fn json_response_body(response: &TransportResponse) -> Result<&Value, ApiProtocolError> {
    match &response.body {
        ResponseBody::Json { value } => Ok(value),
        _ => Err(ApiProtocolError::InvalidShape(
            "expected JSON response body".into(),
        )),
    }
}

fn audio_media_type_for_format(format: Option<&str>) -> String {
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

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiProtocolError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiProtocolError::MissingField(field.into()))
}

fn value_as_i32(value: &Value) -> Result<i32, ApiProtocolError> {
    value
        .as_i64()
        .and_then(|number| i32::try_from(number).ok())
        .ok_or_else(|| ApiProtocolError::InvalidShape("expected integer token value".into()))
}

fn collect_vendor_extensions(value: &Value, known_fields: &[&str]) -> VendorExtensions {
    let Some(object) = value.as_object() else {
        return VendorExtensions::new();
    };

    object
        .iter()
        .filter(|(key, _)| !known_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn extend_with_vendor_extensions(
    map: &mut Map<String, Value>,
    vendor_extensions: &VendorExtensions,
) {
    for (key, value) in vendor_extensions {
        map.entry(key.clone()).or_insert_with(|| value.clone());
    }
}

fn wire_format_name(wire_format: WireFormat) -> &'static str {
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

fn dedupe_loss_reasons(mut reasons: Vec<String>) -> Vec<String> {
    reasons.sort();
    reasons.dedup();
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        CapabilitySet, GenerationConfig, Message, MessageRole, RequestItem, ToolDefinition,
    };

    #[test]
    fn emit_transport_request_for_embeddings_uses_expected_path() {
        let request = ApiRequest::Embeddings(EmbeddingRequest {
            model: "text-embedding-3-small".into(),
            input: vec![EmbeddingInput::Text {
                text: "hello".into(),
            }],
            dimensions: None,
            encoding_format: None,
            user: None,
            vendor_extensions: VendorExtensions::new(),
        });

        let report =
            emit_transport_request(WireFormat::OpenAiEmbeddings, &request).expect("emit transport");

        assert_eq!(report.value.path, "/embeddings");
        let RequestBody::Json { value } = report.value.body else {
            panic!("expected json body");
        };
        assert_eq!(value["input"], "hello");
    }

    #[test]
    fn emit_api_request_degrades_generation_capabilities_for_chat() {
        let request = ApiRequest::Responses(LlmRequest {
            model: "gpt-4.1-mini".into(),
            instructions: Some("be concise".into()),
            input: vec![RequestItem::from(Message::text(MessageRole::User, "hi"))],
            messages: Vec::new(),
            capabilities: CapabilitySet {
                tools: vec![ToolDefinition {
                    name: "lookup_weather".into(),
                    description: None,
                    input_schema: json!({"type":"object"}),
                    strict: false,
                    vendor_extensions: VendorExtensions::new(),
                }],
                builtin_tools: vec![BuiltinTool::WebSearch],
                reasoning: Some(crate::types::ReasoningCapability {
                    effort: Some("medium".into()),
                    summary: None,
                    vendor_extensions: VendorExtensions::new(),
                }),
                ..Default::default()
            },
            generation: GenerationConfig::default(),
            metadata: [("trace_id".into(), Value::String("abc".into()))]
                .into_iter()
                .collect(),
            vendor_extensions: VendorExtensions::new(),
        });

        let report =
            emit_api_request(WireFormat::OpenAiChatCompletions, &request).expect("emit request");

        assert!(report.bridged);
        assert!(report.lossy);
        assert!(report
            .loss_reasons
            .iter()
            .any(|reason| reason.contains("builtin tools")));
        assert!(report
            .loss_reasons
            .iter()
            .any(|reason| reason.contains("reasoning settings")));
        assert!(report
            .loss_reasons
            .iter()
            .any(|reason| reason.contains("metadata")));
    }

    #[test]
    fn parse_transport_response_for_audio_speech_reads_binary_payload() {
        let response = TransportResponse {
            status: 200,
            headers: Default::default(),
            content_type: Some("audio/mpeg".into()),
            body: ResponseBody::Binary {
                data_base64: "ZmFrZQ==".into(),
                media_type: None,
            },
        };

        let report = parse_transport_response(WireFormat::OpenAiAudioSpeech, &response)
            .expect("parse transport response");

        let ApiResponse::AudioSpeech(audio) = report.value else {
            panic!("expected audio speech response");
        };
        assert_eq!(audio.data_base64, "ZmFrZQ==");
        assert_eq!(audio.media_type.as_deref(), Some("audio/mpeg"));
    }

    #[test]
    fn transcode_api_response_from_responses_to_chat_keeps_text() {
        let raw = json!({
            "id": "resp_123",
            "model": "gpt-4.1-mini",
            "status": "stop",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Hello back!" }]
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "total_tokens": 15
            }
        })
        .to_string();

        let report = transcode_api_response(
            WireFormat::OpenAiResponses,
            WireFormat::OpenAiChatCompletions,
            &raw,
        )
        .expect("transcode response");

        let body: Value = serde_json::from_str(&report.value).expect("parse response");
        assert_eq!(body["choices"][0]["message"]["content"], "Hello back!");
    }

    #[test]
    fn parse_and_emit_rerank_round_trips_documents() {
        let raw = json!({
            "model": "rerank-v1",
            "query": "rust",
            "documents": ["Rust Book", {"title":"Cargo"}],
            "top_n": 2,
            "return_documents": true
        })
        .to_string();

        let parsed =
            parse_api_request(WireFormat::OpenAiRerank, &raw).expect("parse rerank request");
        let ApiRequest::Rerank(request) = &parsed.value else {
            panic!("expected rerank request");
        };
        assert_eq!(request.documents.len(), 2);

        let emitted =
            emit_api_request(WireFormat::OpenAiRerank, &parsed.value).expect("emit rerank");
        let body: Value = serde_json::from_str(&emitted.value).expect("parse emitted rerank");
        assert_eq!(body["documents"][0], "Rust Book");
        assert_eq!(body["documents"][1]["title"], "Cargo");
    }

    #[test]
    fn emit_audio_transcription_transport_uses_multipart() {
        let request = ApiRequest::AudioTranscriptions(AudioTranscriptionRequest {
            model: "whisper-1".into(),
            audio: AudioInput::File {
                filename: "clip.wav".into(),
                data_base64: "ZmFrZQ==".into(),
                media_type: Some("audio/wav".into()),
            },
            prompt: None,
            response_format: Some("verbose_json".into()),
            language: Some("en".into()),
            temperature: Some(0.0),
            timestamp_granularities: vec!["word".into()],
            vendor_extensions: VendorExtensions::new(),
        });

        let report = emit_transport_request(WireFormat::OpenAiAudioTranscriptions, &request)
            .expect("emit audio transport");

        let RequestBody::Multipart { fields } = report.value.body else {
            panic!("expected multipart body");
        };
        assert!(fields.iter().any(|field| field.name == "file"));
        assert!(fields
            .iter()
            .any(|field| field.name == "timestamp_granularities[]"));
    }
}
