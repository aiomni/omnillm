use crate::provider_registry::SupportLevel;

use super::{
    PrimitiveBudgetClass, PrimitiveEndpointKind, PrimitiveEndpointSupport,
    PrimitiveProviderDescriptor, PrimitiveProviderKind, PrimitiveProviderRegistry,
    PrimitiveStreamMode, PrimitiveSupportTier, ProviderPrimitiveWireFormat,
};

pub fn embedded_primitive_provider_registry() -> PrimitiveProviderRegistry {
    PrimitiveProviderRegistry {
        providers: vec![
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::OpenAi,
                default_base_url: Some("https://api.openai.com/v1".into()),
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::Responses,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiResponses],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::ChatCompletions,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiChatCompletions],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::Images,
                        SupportLevel::Native,
                        &[
                            ProviderPrimitiveWireFormat::OpenAiImages,
                            ProviderPrimitiveWireFormat::OpenAiImageEdits,
                            ProviderPrimitiveWireFormat::OpenAiImageVariations,
                        ],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Realtime,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiRealtime],
                        &[PrimitiveStreamMode::WebSocket, PrimitiveStreamMode::WebRtc],
                    ),
                    support(
                        PrimitiveEndpointKind::AudioTranscriptions,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiAudioTranscriptions],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::AudioTranslations,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiAudioTranslations],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::AudioSpeech,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiAudioSpeech],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::BinaryChunks],
                    ),
                    support(
                        PrimitiveEndpointKind::Embeddings,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiEmbeddings],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Files,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiFiles],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Uploads,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiUploads],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Models,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiModels],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Batches,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::OpenAiBatches],
                        &[PrimitiveStreamMode::None],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::Anthropic,
                default_base_url: Some("https://api.anthropic.com/v1".into()),
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::Messages,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::AnthropicMessages],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::CountTokens,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::AnthropicCountTokens],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Batches,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::AnthropicMessageBatches],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Files,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::AnthropicFiles],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Models,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::AnthropicModels],
                        &[PrimitiveStreamMode::None],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::Gemini,
                default_base_url: Some("https://generativelanguage.googleapis.com/v1beta".into()),
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::Messages,
                        SupportLevel::Native,
                        &[
                            ProviderPrimitiveWireFormat::GeminiGenerateContent,
                            ProviderPrimitiveWireFormat::GeminiStreamGenerateContent,
                        ],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::CountTokens,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiCountTokens],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Embeddings,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiEmbedContent],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Live,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiLive],
                        &[PrimitiveStreamMode::WebSocket],
                    ),
                    support(
                        PrimitiveEndpointKind::Files,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiFiles],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Caches,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiCaches],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Models,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiModels],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Operations,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiOperations],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Batches,
                        SupportLevel::Native,
                        &[ProviderPrimitiveWireFormat::GeminiBatches],
                        &[PrimitiveStreamMode::None],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::AzureOpenAi,
                default_base_url: None,
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::Responses,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiResponses],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::ChatCompletions,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiChatCompletions],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::Images,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiImages],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::AudioTranscriptions,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiAudioTranscriptions],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::AudioSpeech,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiAudioSpeech],
                        &[PrimitiveStreamMode::None],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::VertexAi,
                default_base_url: None,
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::Messages,
                        SupportLevel::Compatible,
                        &[
                            ProviderPrimitiveWireFormat::GeminiGenerateContent,
                            ProviderPrimitiveWireFormat::GeminiStreamGenerateContent,
                        ],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::CountTokens,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::GeminiCountTokens],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Embeddings,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::GeminiEmbedContent],
                        &[PrimitiveStreamMode::None],
                    ),
                    support(
                        PrimitiveEndpointKind::Rerank,
                        SupportLevel::Planned,
                        &[],
                        &[],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::Bedrock,
                default_base_url: None,
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::Messages,
                        SupportLevel::Planned,
                        &[],
                        &[],
                    ),
                    support(
                        PrimitiveEndpointKind::Custom,
                        SupportLevel::Planned,
                        &[],
                        &[],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::OpenAiCompatible,
                default_base_url: None,
                endpoints: vec![
                    support(
                        PrimitiveEndpointKind::ChatCompletions,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::Responses,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiResponses],
                        &[PrimitiveStreamMode::None, PrimitiveStreamMode::Sse],
                    ),
                    support(
                        PrimitiveEndpointKind::Embeddings,
                        SupportLevel::Compatible,
                        &[ProviderPrimitiveWireFormat::OpenAiEmbeddings],
                        &[PrimitiveStreamMode::None],
                    ),
                ],
            },
            PrimitiveProviderDescriptor {
                kind: PrimitiveProviderKind::Custom,
                default_base_url: None,
                endpoints: vec![support(
                    PrimitiveEndpointKind::Custom,
                    SupportLevel::Compatible,
                    &[ProviderPrimitiveWireFormat::CustomHttp],
                    &[
                        PrimitiveStreamMode::None,
                        PrimitiveStreamMode::Sse,
                        PrimitiveStreamMode::BinaryChunks,
                    ],
                )],
            },
        ],
    }
}

fn support(
    endpoint: PrimitiveEndpointKind,
    level: SupportLevel,
    wire_formats: &[ProviderPrimitiveWireFormat],
    stream_modes: &[PrimitiveStreamMode],
) -> PrimitiveEndpointSupport {
    PrimitiveEndpointSupport {
        endpoint,
        level,
        wire_formats: wire_formats.to_vec(),
        stream_modes: stream_modes.to_vec(),
        scope_tier: infer_scope_tier(endpoint, wire_formats, stream_modes),
        budget_class: infer_budget_class(endpoint, wire_formats),
    }
}

fn infer_scope_tier(
    endpoint: PrimitiveEndpointKind,
    wire_formats: &[ProviderPrimitiveWireFormat],
    stream_modes: &[PrimitiveStreamMode],
) -> PrimitiveSupportTier {
    if stream_modes.iter().any(|mode| {
        matches!(
            mode,
            PrimitiveStreamMode::WebSocket
                | PrimitiveStreamMode::WebRtc
                | PrimitiveStreamMode::BinaryChunks
        )
    }) {
        return PrimitiveSupportTier::P3TransportExpansion;
    }

    if matches!(endpoint, PrimitiveEndpointKind::Batches) {
        return PrimitiveSupportTier::P2AsyncJobLifecycle;
    }

    if matches!(
        endpoint,
        PrimitiveEndpointKind::Files
            | PrimitiveEndpointKind::Models
            | PrimitiveEndpointKind::Operations
            | PrimitiveEndpointKind::Caches
            | PrimitiveEndpointKind::Uploads
    ) {
        return PrimitiveSupportTier::P1LowRiskHttpGaps;
    }

    if wire_formats.iter().any(|wire_format| {
        matches!(
            wire_format,
            ProviderPrimitiveWireFormat::GeminiFiles
                | ProviderPrimitiveWireFormat::GeminiCaches
                | ProviderPrimitiveWireFormat::AnthropicFiles
        )
    }) {
        return PrimitiveSupportTier::P1LowRiskHttpGaps;
    }

    PrimitiveSupportTier::P0KeepAndHarden
}

pub(super) fn infer_budget_class(
    endpoint: PrimitiveEndpointKind,
    wire_formats: &[ProviderPrimitiveWireFormat],
) -> PrimitiveBudgetClass {
    if matches!(
        endpoint,
        PrimitiveEndpointKind::Models
            | PrimitiveEndpointKind::Operations
            | PrimitiveEndpointKind::Caches
    ) {
        return PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost;
    }

    if matches!(
        endpoint,
        PrimitiveEndpointKind::Files | PrimitiveEndpointKind::Uploads
    ) {
        return PrimitiveBudgetClass::UploadOrStorage;
    }

    if matches!(
        endpoint,
        PrimitiveEndpointKind::Images
            | PrimitiveEndpointKind::AudioTranscriptions
            | PrimitiveEndpointKind::AudioSpeech
    ) {
        return PrimitiveBudgetClass::BillableUnitMetered;
    }

    if wire_formats.iter().any(|wire_format| {
        matches!(
            wire_format,
            ProviderPrimitiveWireFormat::OpenAiImages
                | ProviderPrimitiveWireFormat::OpenAiImageEdits
                | ProviderPrimitiveWireFormat::OpenAiImageVariations
                | ProviderPrimitiveWireFormat::OpenAiAudioTranscriptions
                | ProviderPrimitiveWireFormat::OpenAiAudioTranslations
                | ProviderPrimitiveWireFormat::OpenAiAudioSpeech
        )
    }) {
        return PrimitiveBudgetClass::BillableUnitMetered;
    }

    PrimitiveBudgetClass::TokenMetered
}
