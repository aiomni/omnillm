use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{HttpMethod, RequestBody, ResponseBody};
use crate::protocol::{AuthScheme, ProviderEndpoint, ProviderProtocol};
use crate::provider_registry::SupportLevel;
use crate::types::{PromptCacheUsage, TokenUsage, VendorExtensions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveSupportTier {
    P0KeepAndHarden,
    P1LowRiskHttpGaps,
    P2AsyncJobLifecycle,
    P3TransportExpansion,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveBudgetClass {
    TokenMetered,
    BillableUnitMetered,
    MetadataOrControlPlaneZeroCost,
    UploadOrStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveProviderKind {
    OpenAi,
    AzureOpenAi,
    Anthropic,
    Gemini,
    VertexAi,
    Bedrock,
    OpenAiCompatible,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveEndpointKind {
    Responses,
    ChatCompletions,
    Images,
    Realtime,
    AudioTranscriptions,
    AudioTranslations,
    AudioSpeech,
    Embeddings,
    Uploads,
    Messages,
    CountTokens,
    Batches,
    Files,
    Caches,
    Models,
    Live,
    Rerank,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPrimitiveWireFormat {
    OpenAiResponses,
    OpenAiChatCompletions,
    OpenAiImages,
    OpenAiRealtime,
    OpenAiImageEdits,
    OpenAiImageVariations,
    OpenAiAudioTranscriptions,
    OpenAiAudioTranslations,
    OpenAiAudioSpeech,
    OpenAiEmbeddings,
    OpenAiFiles,
    OpenAiUploads,
    OpenAiModels,
    AnthropicMessages,
    AnthropicCountTokens,
    AnthropicMessageBatches,
    AnthropicFiles,
    GeminiGenerateContent,
    GeminiStreamGenerateContent,
    GeminiCountTokens,
    GeminiEmbedContent,
    GeminiLive,
    GeminiFiles,
    GeminiCaches,
    BedrockConverse,
    BedrockInvokeModel,
    OpenAiCompatibleChatCompletions,
    CustomHttp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveStreamMode {
    None,
    Sse,
    WebSocket,
    WebRtc,
    BinaryChunks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveBillableUnit {
    pub name: String,
    pub amount: u64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveUsageTelemetry {
    pub raw_usage: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub billable_units: Vec<PrimitiveBillableUnit>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveRequest {
    pub provider: PrimitiveProviderKind,
    pub endpoint: PrimitiveEndpointKind,
    pub wire_format: ProviderPrimitiveWireFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub method: HttpMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<String>,
    pub body: RequestBody,
    pub stream: PrimitiveStreamMode,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: VendorExtensions,
}

impl PrimitiveRequest {
    pub fn json(
        provider: PrimitiveProviderKind,
        endpoint: PrimitiveEndpointKind,
        wire_format: ProviderPrimitiveWireFormat,
        model: impl Into<String>,
        value: Value,
    ) -> Self {
        Self {
            provider,
            endpoint,
            wire_format,
            model: Some(model.into()),
            method: HttpMethod::Post,
            path: None,
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            accept: None,
            body: RequestBody::Json { value },
            stream: PrimitiveStreamMode::None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn get(
        provider: PrimitiveProviderKind,
        endpoint: PrimitiveEndpointKind,
        wire_format: ProviderPrimitiveWireFormat,
        model: Option<impl Into<String>>,
    ) -> Self {
        Self {
            provider,
            endpoint,
            wire_format,
            model: model.map(Into::into),
            method: HttpMethod::Get,
            path: None,
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            accept: None,
            body: RequestBody::Text {
                text: String::new(),
            },
            stream: PrimitiveStreamMode::None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn budget_class(&self) -> PrimitiveBudgetClass {
        infer_budget_class(self.endpoint, &[self.wire_format])
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(name.into(), value.into());
        self
    }

    pub fn with_stream(mut self, stream: PrimitiveStreamMode) -> Self {
        self.stream = stream;
        self
    }

    pub fn estimated_tokens(&self) -> u32 {
        let prompt_tokens = self.estimated_prompt_tokens();
        let output_tokens = self
            .json_body()
            .and_then(known_output_token_limit)
            .unwrap_or(1024);
        prompt_tokens.saturating_add(output_tokens).max(1)
    }

    pub fn estimated_prompt_tokens(&self) -> u32 {
        estimate_body_tokens(&self.body).max(1)
    }

    pub fn model_name(&self) -> &str {
        self.model.as_deref().unwrap_or("unknown-model")
    }

    pub fn json_body(&self) -> Option<&Value> {
        match &self.body {
            RequestBody::Json { value } => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveResponse {
    pub provider: PrimitiveProviderKind,
    pub endpoint: PrimitiveEndpointKind,
    pub wire_format: ProviderPrimitiveWireFormat,
    pub status: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub body: ResponseBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<PrimitiveUsageTelemetry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: VendorExtensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PrimitiveStreamEvent {
    SseFrame {
        #[serde(skip_serializing_if = "Option::is_none")]
        event: Option<String>,
        data: String,
    },
    WebSocketMessage {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data_base64: Option<String>,
    },
    BinaryChunk {
        data_base64: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
    Usage {
        usage: PrimitiveUsageTelemetry,
    },
    Completed {
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<PrimitiveUsageTelemetry>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveRealtimeSession {
    pub provider: PrimitiveProviderKind,
    pub endpoint: PrimitiveEndpointKind,
    pub wire_format: ProviderPrimitiveWireFormat,
    pub stream_mode: PrimitiveStreamMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveProviderEndpoint {
    pub provider: PrimitiveProviderKind,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthScheme>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub default_headers: BTreeMap<String, String>,
}

impl PrimitiveProviderEndpoint {
    pub fn new(provider: PrimitiveProviderKind, base_url: impl Into<String>) -> Self {
        Self {
            provider,
            base_url: base_url.into(),
            auth: None,
            default_headers: default_headers_for(provider),
        }
    }

    pub fn openai() -> Self {
        Self::new(PrimitiveProviderKind::OpenAi, "https://api.openai.com/v1")
    }

    pub fn anthropic() -> Self {
        Self::new(
            PrimitiveProviderKind::Anthropic,
            "https://api.anthropic.com/v1",
        )
    }

    pub fn gemini() -> Self {
        Self::new(
            PrimitiveProviderKind::Gemini,
            "https://generativelanguage.googleapis.com/v1beta",
        )
    }

    pub fn openai_compatible(base_url: impl Into<String>) -> Self {
        Self::new(PrimitiveProviderKind::OpenAiCompatible, base_url)
    }

    pub fn custom(base_url: impl Into<String>) -> Self {
        Self::new(PrimitiveProviderKind::Custom, base_url)
    }

    pub fn with_auth(mut self, auth: AuthScheme) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_default_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    pub fn auth_scheme(&self) -> AuthScheme {
        self.auth
            .clone()
            .unwrap_or_else(|| default_auth_for(self.provider))
    }

    pub fn request_url(&self, request: &PrimitiveRequest) -> Result<String, String> {
        let path = match &request.path {
            Some(path) if path.starts_with("http://") || path.starts_with("https://") => {
                return Ok(path.clone());
            }
            Some(path) => path.clone(),
            None => default_path(request).ok_or_else(|| {
                format!(
                    "primitive request for {:?}/{:?} requires an explicit path",
                    request.provider, request.wire_format
                )
            })?,
        };

        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        Ok(format!("{base}/{path}"))
    }

    pub fn supports(&self, request: &PrimitiveRequest) -> bool {
        if self.provider != request.provider && self.provider != PrimitiveProviderKind::Custom {
            return false;
        }
        embedded_primitive_provider_registry().supports_request(request)
    }
}

impl From<&ProviderEndpoint> for PrimitiveProviderEndpoint {
    fn from(value: &ProviderEndpoint) -> Self {
        let provider = match value.wire_protocol() {
            ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
                PrimitiveProviderKind::OpenAi
            }
            ProviderProtocol::ClaudeMessages => PrimitiveProviderKind::Anthropic,
            ProviderProtocol::GeminiGenerateContent => PrimitiveProviderKind::Gemini,
        };
        Self {
            provider,
            base_url: value.base_url.clone(),
            auth: value.auth.clone(),
            default_headers: value.default_headers.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveEndpointSupport {
    pub endpoint: PrimitiveEndpointKind,
    pub level: SupportLevel,
    pub wire_formats: Vec<ProviderPrimitiveWireFormat>,
    pub stream_modes: Vec<PrimitiveStreamMode>,
    pub scope_tier: PrimitiveSupportTier,
    pub budget_class: PrimitiveBudgetClass,
}

impl PrimitiveEndpointSupport {
    pub fn is_enabled(&self) -> bool {
        !matches!(self.level, SupportLevel::Planned) && !self.wire_formats.is_empty()
    }

    pub fn supports_wire_format(
        &self,
        wire_format: ProviderPrimitiveWireFormat,
        stream_mode: PrimitiveStreamMode,
    ) -> bool {
        self.is_enabled()
            && self.wire_formats.contains(&wire_format)
            && self.stream_modes.contains(&stream_mode)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveProviderDescriptor {
    pub kind: PrimitiveProviderKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_base_url: Option<String>,
    pub endpoints: Vec<PrimitiveEndpointSupport>,
}

impl PrimitiveProviderDescriptor {
    pub fn supports_wire_format(
        &self,
        wire_format: ProviderPrimitiveWireFormat,
        stream_mode: PrimitiveStreamMode,
    ) -> bool {
        self.endpoints
            .iter()
            .any(|endpoint| endpoint.supports_wire_format(wire_format, stream_mode))
    }

    pub fn supports_endpoint(&self, endpoint: PrimitiveEndpointKind) -> bool {
        self.endpoints
            .iter()
            .any(|support| support.endpoint == endpoint && support.is_enabled())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveProviderRegistry {
    pub providers: Vec<PrimitiveProviderDescriptor>,
}

impl PrimitiveProviderRegistry {
    pub fn provider(
        &self,
        provider: PrimitiveProviderKind,
    ) -> Option<&PrimitiveProviderDescriptor> {
        self.providers.iter().find(|item| item.kind == provider)
    }

    pub fn supports_endpoint(
        &self,
        provider: PrimitiveProviderKind,
        endpoint: PrimitiveEndpointKind,
    ) -> bool {
        self.provider(provider)
            .map(|descriptor| descriptor.supports_endpoint(endpoint))
            .unwrap_or(false)
    }

    pub fn supports_wire_format(
        &self,
        provider: PrimitiveProviderKind,
        wire_format: ProviderPrimitiveWireFormat,
        stream_mode: PrimitiveStreamMode,
    ) -> bool {
        self.provider(provider)
            .map(|descriptor| descriptor.supports_wire_format(wire_format, stream_mode))
            .unwrap_or(false)
    }

    pub fn supports_request(&self, request: &PrimitiveRequest) -> bool {
        self.provider(request.provider)
            .map(|descriptor| {
                descriptor.endpoints.iter().any(|support| {
                    support.endpoint == request.endpoint
                        && support.supports_wire_format(request.wire_format, request.stream)
                })
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveProviderError {
    pub provider: PrimitiveProviderKind,
    pub wire_format: ProviderPrimitiveWireFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl fmt::Display for PrimitiveProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?}/{:?} error",
            self.provider, self.wire_format
        )?;
        if let Some(status) = self.status {
            write!(formatter, " ({status})")?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl Error for PrimitiveProviderError {}

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

pub(crate) fn extract_usage(
    wire_format: ProviderPrimitiveWireFormat,
    body: &ResponseBody,
) -> Option<PrimitiveUsageTelemetry> {
    let ResponseBody::Json { value } = body else {
        return None;
    };

    let usage = match wire_format {
        ProviderPrimitiveWireFormat::OpenAiResponses
        | ProviderPrimitiveWireFormat::OpenAiChatCompletions
        | ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions => value.get("usage"),
        ProviderPrimitiveWireFormat::AnthropicMessages => value.get("usage"),
        ProviderPrimitiveWireFormat::GeminiGenerateContent
        | ProviderPrimitiveWireFormat::GeminiStreamGenerateContent => value.get("usageMetadata"),
        _ => value.get("usage"),
    }?;

    let token_usage = token_usage_from_raw(wire_format, usage);
    Some(PrimitiveUsageTelemetry {
        raw_usage: usage.clone(),
        token_usage,
        billable_units: Vec::new(),
        vendor_extensions: BTreeMap::new(),
    })
}

pub(crate) fn primitive_error_from_body(
    provider: PrimitiveProviderKind,
    wire_format: ProviderPrimitiveWireFormat,
    status: Option<u16>,
    retry_after: Option<Duration>,
    raw_body: String,
) -> PrimitiveProviderError {
    let parsed = serde_json::from_str::<Value>(&raw_body).ok();
    let code = parsed.as_ref().and_then(extract_error_code);
    let message = parsed
        .as_ref()
        .and_then(extract_error_message)
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| raw_body.clone());

    PrimitiveProviderError {
        provider,
        wire_format,
        status,
        code,
        message,
        retry_after,
        raw_body: Some(raw_body),
        vendor_extensions: BTreeMap::new(),
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

fn infer_budget_class(
    endpoint: PrimitiveEndpointKind,
    wire_formats: &[ProviderPrimitiveWireFormat],
) -> PrimitiveBudgetClass {
    if matches!(endpoint, PrimitiveEndpointKind::Models) {
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

fn default_headers_for(provider: PrimitiveProviderKind) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    if matches!(provider, PrimitiveProviderKind::Anthropic) {
        headers.insert("anthropic-version".into(), "2023-06-01".into());
    }
    headers
}

fn default_auth_for(provider: PrimitiveProviderKind) -> AuthScheme {
    match provider {
        PrimitiveProviderKind::OpenAi
        | PrimitiveProviderKind::AzureOpenAi
        | PrimitiveProviderKind::OpenAiCompatible
        | PrimitiveProviderKind::Bedrock
        | PrimitiveProviderKind::Custom => AuthScheme::Bearer,
        PrimitiveProviderKind::Anthropic => AuthScheme::Header {
            name: "x-api-key".into(),
        },
        PrimitiveProviderKind::Gemini | PrimitiveProviderKind::VertexAi => AuthScheme::Header {
            name: "x-goog-api-key".into(),
        },
    }
}

fn default_path(request: &PrimitiveRequest) -> Option<String> {
    match request.wire_format {
        ProviderPrimitiveWireFormat::OpenAiResponses => Some("/responses".into()),
        ProviderPrimitiveWireFormat::OpenAiChatCompletions
        | ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions => {
            Some("/chat/completions".into())
        }
        ProviderPrimitiveWireFormat::OpenAiImages => Some("/images/generations".into()),
        ProviderPrimitiveWireFormat::OpenAiImageEdits => Some("/images/edits".into()),
        ProviderPrimitiveWireFormat::OpenAiImageVariations => Some("/images/variations".into()),
        ProviderPrimitiveWireFormat::OpenAiRealtime => Some("/realtime/sessions".into()),
        ProviderPrimitiveWireFormat::OpenAiAudioTranscriptions => {
            Some("/audio/transcriptions".into())
        }
        ProviderPrimitiveWireFormat::OpenAiAudioTranslations => Some("/audio/translations".into()),
        ProviderPrimitiveWireFormat::OpenAiAudioSpeech => Some("/audio/speech".into()),
        ProviderPrimitiveWireFormat::OpenAiEmbeddings => Some("/embeddings".into()),
        ProviderPrimitiveWireFormat::OpenAiFiles => Some("/files".into()),
        ProviderPrimitiveWireFormat::OpenAiUploads => Some("/uploads".into()),
        ProviderPrimitiveWireFormat::OpenAiModels => Some("/models".into()),
        ProviderPrimitiveWireFormat::AnthropicMessages => Some("/messages".into()),
        ProviderPrimitiveWireFormat::AnthropicCountTokens => Some("/messages/count_tokens".into()),
        ProviderPrimitiveWireFormat::AnthropicMessageBatches => Some("/messages/batches".into()),
        ProviderPrimitiveWireFormat::AnthropicFiles => Some("/files".into()),
        ProviderPrimitiveWireFormat::GeminiGenerateContent => {
            model_path(request, "generateContent")
        }
        ProviderPrimitiveWireFormat::GeminiStreamGenerateContent => {
            model_path(request, "streamGenerateContent")
        }
        ProviderPrimitiveWireFormat::GeminiCountTokens => model_path(request, "countTokens"),
        ProviderPrimitiveWireFormat::GeminiEmbedContent => model_path(request, "embedContent"),
        ProviderPrimitiveWireFormat::GeminiLive => None,
        ProviderPrimitiveWireFormat::GeminiFiles => Some("/files".into()),
        ProviderPrimitiveWireFormat::GeminiCaches => Some("/cachedContents".into()),
        ProviderPrimitiveWireFormat::BedrockConverse
        | ProviderPrimitiveWireFormat::BedrockInvokeModel
        | ProviderPrimitiveWireFormat::CustomHttp => None,
    }
}

fn model_path(request: &PrimitiveRequest, action: &str) -> Option<String> {
    let model = request.model.as_ref()?;
    Some(format!("/models/{model}:{action}"))
}

fn estimate_body_tokens(body: &RequestBody) -> u32 {
    let chars = match body {
        RequestBody::Json { value } => value.to_string().len(),
        RequestBody::Multipart { fields } => fields
            .iter()
            .map(|field| match &field.value {
                crate::api::MultipartValue::Text { value } => value.len(),
                crate::api::MultipartValue::File { data_base64, .. } => data_base64.len() / 8,
            })
            .sum(),
        RequestBody::Text { text } => text.len(),
        RequestBody::Binary { data_base64, .. } => data_base64.len() / 8,
    };
    ((chars / 4).max(1)) as u32
}

fn known_output_token_limit(value: &Value) -> Option<u32> {
    value
        .get("max_output_tokens")
        .or_else(|| value.get("max_tokens"))
        .or_else(|| value.get("maxOutputTokens"))
        .or_else(|| value.pointer("/generationConfig/maxOutputTokens"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn token_usage_from_raw(
    wire_format: ProviderPrimitiveWireFormat,
    usage: &Value,
) -> Option<TokenUsage> {
    match wire_format {
        ProviderPrimitiveWireFormat::OpenAiResponses => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["input_tokens"]),
            completion_tokens: usage_u32(usage, &["output_tokens"]),
            total_tokens: usage_u32_opt(usage, &["total_tokens"]),
            prompt_cache: openai_prompt_cache_usage(usage),
        }),
        ProviderPrimitiveWireFormat::OpenAiChatCompletions
        | ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["prompt_tokens"]),
            completion_tokens: usage_u32(usage, &["completion_tokens"]),
            total_tokens: usage_u32_opt(usage, &["total_tokens"]),
            prompt_cache: openai_prompt_cache_usage(usage),
        }),
        ProviderPrimitiveWireFormat::AnthropicMessages => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["input_tokens"]),
            completion_tokens: usage_u32(usage, &["output_tokens"]),
            total_tokens: None,
            prompt_cache: anthropic_prompt_cache_usage(usage),
        }),
        ProviderPrimitiveWireFormat::GeminiGenerateContent
        | ProviderPrimitiveWireFormat::GeminiStreamGenerateContent => Some(TokenUsage {
            prompt_tokens: usage_u32(usage, &["promptTokenCount"]),
            completion_tokens: usage_u32(usage, &["candidatesTokenCount"]),
            total_tokens: usage_u32_opt(usage, &["totalTokenCount"]),
            prompt_cache: None,
        }),
        _ => generic_token_usage(usage),
    }
}

fn generic_token_usage(usage: &Value) -> Option<TokenUsage> {
    let prompt_tokens = usage_u32(
        usage,
        &[
            "input_tokens",
            "prompt_tokens",
            "promptTokenCount",
            "total_tokens",
        ],
    );
    let completion_tokens = usage_u32(
        usage,
        &["output_tokens", "completion_tokens", "candidatesTokenCount"],
    );
    if prompt_tokens == 0 && completion_tokens == 0 {
        return None;
    }
    Some(TokenUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens: usage_u32_opt(usage, &["total_tokens", "totalTokenCount"]),
        prompt_cache: None,
    })
}

fn openai_prompt_cache_usage(usage: &Value) -> Option<PromptCacheUsage> {
    let cached_input_tokens = usage
        .pointer("/input_tokens_details/cached_tokens")
        .or_else(|| usage.pointer("/prompt_tokens_details/cached_tokens"))
        .and_then(value_to_u32);
    cached_input_tokens.map(|cached_input_tokens| PromptCacheUsage {
        cached_input_tokens: Some(cached_input_tokens),
        ..Default::default()
    })
}

fn anthropic_prompt_cache_usage(usage: &Value) -> Option<PromptCacheUsage> {
    let prompt_cache = PromptCacheUsage {
        cache_read_input_tokens: usage.get("cache_read_input_tokens").and_then(value_to_u32),
        cache_creation_input_tokens: usage
            .get("cache_creation_input_tokens")
            .and_then(value_to_u32),
        cache_creation_short_input_tokens: usage
            .get("cache_creation_5m_input_tokens")
            .or_else(|| usage.pointer("/cache_creation/ephemeral_5m_input_tokens"))
            .and_then(value_to_u32),
        cache_creation_long_input_tokens: usage
            .get("cache_creation_1h_input_tokens")
            .or_else(|| usage.pointer("/cache_creation/ephemeral_1h_input_tokens"))
            .and_then(value_to_u32),
        ..Default::default()
    };
    if prompt_cache.cached_input_tokens.is_some()
        || prompt_cache.cache_read_input_tokens.is_some()
        || prompt_cache.cache_creation_input_tokens.is_some()
        || prompt_cache.cache_creation_short_input_tokens.is_some()
        || prompt_cache.cache_creation_long_input_tokens.is_some()
    {
        Some(prompt_cache)
    } else {
        None
    }
}

fn usage_u32(usage: &Value, fields: &[&str]) -> u32 {
    usage_u32_opt(usage, fields).unwrap_or(0)
}

fn usage_u32_opt(usage: &Value, fields: &[&str]) -> Option<u32> {
    fields
        .iter()
        .find_map(|field| usage.get(*field).and_then(value_to_u32))
}

fn value_to_u32(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

fn extract_error_code(value: &Value) -> Option<String> {
    value
        .pointer("/error/code")
        .or_else(|| value.pointer("/error/type"))
        .or_else(|| value.get("code"))
        .or_else(|| value.get("type"))
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
}

fn extract_error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}
