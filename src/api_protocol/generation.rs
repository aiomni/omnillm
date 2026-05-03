use crate::api::{
    ApiRequest, ApiResponse, ConversionReport, EndpointKind, TransportRequest, WireFormat,
};
use crate::protocol::ProviderProtocol;
use crate::types::{BuiltinTool, CacheBreakpoint, LlmRequest, MessageRole, PromptCachePolicy};

use super::common::*;
use super::ApiProtocolError;

pub(super) fn generation_protocol(
    wire_format: WireFormat,
) -> Result<ProviderProtocol, ApiProtocolError> {
    ProviderProtocol::try_from(wire_format).map_err(|_| ApiProtocolError::UnsupportedWireFormat {
        endpoint: EndpointKind::Responses,
        wire_format,
    })
}

pub(super) fn ensure_matching_endpoint(
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

pub(super) fn generation_request_report(
    wire_format: WireFormat,
    value: ApiRequest,
    loss_reasons: Vec<String>,
) -> ConversionReport<ApiRequest> {
    if wire_format == WireFormat::OpenAiResponses && loss_reasons.is_empty() {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

pub(super) fn generation_response_report(
    wire_format: WireFormat,
    value: ApiResponse,
    loss_reasons: Vec<String>,
) -> ConversionReport<ApiResponse> {
    if wire_format == WireFormat::OpenAiResponses && loss_reasons.is_empty() {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

pub(super) fn generation_string_report(
    wire_format: WireFormat,
    value: String,
    loss_reasons: Vec<String>,
) -> ConversionReport<String> {
    if wire_format == WireFormat::OpenAiResponses && loss_reasons.is_empty() {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

pub(super) fn generation_transport_report(
    wire_format: WireFormat,
    value: TransportRequest,
    loss_reasons: Vec<String>,
) -> ConversionReport<TransportRequest> {
    if wire_format == WireFormat::OpenAiResponses && loss_reasons.is_empty() {
        ConversionReport::native(value, EndpointKind::Responses, wire_format)
    } else {
        ConversionReport::bridged(value, EndpointKind::Responses, wire_format, loss_reasons)
    }
}

pub(super) fn sanitize_generation_request(
    wire_format: WireFormat,
    request: &LlmRequest,
) -> Result<(LlmRequest, Vec<String>), ApiProtocolError> {
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

    sanitize_prompt_cache_policy(wire_format, &mut sanitized, &mut loss_reasons)?;

    if request_has_unemitted_vendor_extensions(wire_format, request) {
        loss_reasons.push(format!(
            "some vendor_extensions and raw_message fields are not emitted to {}",
            wire_format_name(wire_format)
        ));
    }

    Ok((sanitized, dedupe_loss_reasons(loss_reasons)))
}

pub(super) fn sanitize_prompt_cache_policy(
    wire_format: WireFormat,
    request: &mut LlmRequest,
    loss_reasons: &mut Vec<String>,
) -> Result<(), ApiProtocolError> {
    let Some(policy) = request.capabilities.effective_prompt_cache() else {
        return Ok(());
    };
    if policy.is_disabled() {
        return Ok(());
    }

    match wire_format {
        WireFormat::OpenAiResponses | WireFormat::OpenAiChatCompletions => {
            if !policy.breakpoint().is_auto() {
                if policy.is_required() {
                    return unsupported_prompt_cache(
                        wire_format,
                        "OpenAI prompt cache does not support explicit breakpoints",
                    );
                }
                loss_reasons.push(format!(
                    "prompt cache breakpoint is not emitted when bridging to {}",
                    wire_format_name(wire_format)
                ));
            }
        }
        WireFormat::AnthropicMessages => {
            if policy.key().is_some() {
                if policy.is_required() {
                    return unsupported_prompt_cache(
                        wire_format,
                        "Claude prompt cache does not support explicit cache keys",
                    );
                }
                loss_reasons
                    .push("prompt cache key is dropped when bridging to anthropic_messages".into());
            }
            if !claude_prompt_cache_placement_available(&policy, request) {
                if policy.is_required() {
                    return unsupported_prompt_cache(
                        wire_format,
                        "Claude prompt cache breakpoint cannot be represented for this request",
                    );
                }
                clear_prompt_cache_policy(request);
                loss_reasons.push(
                    "prompt cache policy is dropped when bridging to anthropic_messages".into(),
                );
            }
        }
        WireFormat::GeminiGenerateContent => {
            if policy.is_required() {
                return unsupported_prompt_cache(
                    wire_format,
                    "prompt cache is not supported by gemini_generate_content",
                );
            }
            clear_prompt_cache_policy(request);
            loss_reasons.push(
                "prompt cache policy is dropped when bridging to gemini_generate_content".into(),
            );
        }
        _ => {
            if policy.is_required() {
                return unsupported_prompt_cache(
                    wire_format,
                    "prompt cache is only supported for generation wire formats",
                );
            }
            clear_prompt_cache_policy(request);
            loss_reasons.push(format!(
                "prompt cache policy is dropped when bridging to {}",
                wire_format_name(wire_format)
            ));
        }
    }

    Ok(())
}

pub(super) fn unsupported_prompt_cache<T>(
    wire_format: WireFormat,
    message: impl Into<String>,
) -> Result<T, ApiProtocolError> {
    Err(ApiProtocolError::UnsupportedFeature {
        wire_format,
        message: message.into(),
    })
}

pub(super) fn clear_prompt_cache_policy(request: &mut LlmRequest) {
    request.capabilities.prompt_cache = None;
    request.capabilities.cache = None;
}

pub(super) fn claude_prompt_cache_placement_available(
    policy: &PromptCachePolicy,
    request: &LlmRequest,
) -> bool {
    match policy.breakpoint() {
        CacheBreakpoint::Auto => {
            !request.capabilities.tools.is_empty() || request.normalized_instructions().is_some()
        }
        CacheBreakpoint::EndOfTools => !request.capabilities.tools.is_empty(),
        CacheBreakpoint::EndOfInstructions => request.normalized_instructions().is_some(),
        CacheBreakpoint::EndOfMessage { index } => request
            .normalized_messages()
            .into_iter()
            .filter(|message| !matches!(message.role, MessageRole::System | MessageRole::Developer))
            .nth(index)
            .is_some_and(|message| !message.parts.is_empty()),
        CacheBreakpoint::EndOfContentBlock {
            message_index,
            part_index,
        } => request
            .normalized_messages()
            .into_iter()
            .filter(|message| !matches!(message.role, MessageRole::System | MessageRole::Developer))
            .nth(message_index)
            .is_some_and(|message| part_index < message.parts.len()),
    }
}

pub(super) fn request_has_unemitted_vendor_extensions(
    wire_format: WireFormat,
    request: &LlmRequest,
) -> bool {
    let top_level_request_vendor_extensions_are_emitted = matches!(
        wire_format,
        WireFormat::OpenAiResponses | WireFormat::OpenAiChatCompletions
    );

    if (!top_level_request_vendor_extensions_are_emitted && !request.vendor_extensions.is_empty())
        || !request.capabilities.vendor_extensions.is_empty()
        || !request.generation.vendor_extensions.is_empty()
    {
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

pub(super) fn wire_path(wire_format: WireFormat, model: &str) -> String {
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
