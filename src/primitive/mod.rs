use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{HttpMethod, RequestBody, ResponseBody};
use crate::protocol::{AuthScheme, ProviderEndpoint, ProviderProtocol};
use crate::provider_registry::SupportLevel;
use crate::types::{TokenUsage, VendorExtensions};

mod registry;
mod routing;
mod telemetry;

pub use registry::embedded_primitive_provider_registry;
pub(crate) use telemetry::{
    extract_async_job_id, extract_async_job_status, extract_usage, primitive_error_from_body,
};

use registry::infer_budget_class;
use routing::{
    default_auth_for, default_headers_for, default_path, estimate_body_tokens,
    known_output_token_limit,
};

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
    Operations,
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
    OpenAiBatches,
    AnthropicMessages,
    AnthropicCountTokens,
    AnthropicMessageBatches,
    AnthropicFiles,
    AnthropicModels,
    GeminiGenerateContent,
    GeminiStreamGenerateContent,
    GeminiCountTokens,
    GeminiEmbedContent,
    GeminiLive,
    GeminiFiles,
    GeminiCaches,
    GeminiModels,
    GeminiOperations,
    GeminiBatches,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveAsyncJobOperation {
    Create,
    Get,
    List,
    Cancel,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveAsyncJobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveAsyncJobRequest {
    pub operation: PrimitiveAsyncJobOperation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub request: PrimitiveRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveAsyncJobResponse {
    pub operation: PrimitiveAsyncJobOperation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub status: PrimitiveAsyncJobStatus,
    pub response: PrimitiveResponse,
}

impl PrimitiveAsyncJobRequest {
    pub fn new(operation: PrimitiveAsyncJobOperation, request: PrimitiveRequest) -> Self {
        Self {
            operation,
            job_id: None,
            request,
        }
    }

    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn estimated_cost(&self) -> u64 {
        match self.operation {
            PrimitiveAsyncJobOperation::Results => match self.request.budget_class() {
                PrimitiveBudgetClass::MetadataOrControlPlaneZeroCost
                | PrimitiveBudgetClass::UploadOrStorage => 0,
                PrimitiveBudgetClass::TokenMetered | PrimitiveBudgetClass::BillableUnitMetered => {
                    crate::pricing::estimate(
                        self.request.estimated_tokens(),
                        self.request.model_name(),
                    )
                }
            },
            PrimitiveAsyncJobOperation::Create
            | PrimitiveAsyncJobOperation::Get
            | PrimitiveAsyncJobOperation::List
            | PrimitiveAsyncJobOperation::Cancel => 0,
        }
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<PrimitiveStreamEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<PrimitiveUsageTelemetry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: VendorExtensions,
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
