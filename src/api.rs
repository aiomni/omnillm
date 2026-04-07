//! Multi-endpoint API abstractions built around the existing Responses canonical.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::ProviderProtocol;
use crate::types::{LlmRequest, LlmResponse, VendorExtensions};

/// Endpoint families exposed by upstream providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointKind {
    Responses,
    ChatCompletions,
    Messages,
    Embeddings,
    ImageGenerations,
    AudioTranscriptions,
    AudioSpeech,
    Rerank,
}

/// Provider/platform families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    AzureOpenAi,
    Anthropic,
    Gemini,
    VertexAi,
    Bedrock,
    OpenAiCompatible,
}

/// Supported wire formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireFormat {
    OpenAiResponses,
    OpenAiChatCompletions,
    AnthropicMessages,
    GeminiGenerateContent,
    OpenAiEmbeddings,
    OpenAiImageGenerations,
    OpenAiAudioTranscriptions,
    OpenAiAudioSpeech,
    OpenAiRerank,
}

impl WireFormat {
    /// Returns the upstream endpoint family represented by this wire format.
    pub fn wire_endpoint_kind(self) -> EndpointKind {
        match self {
            Self::OpenAiResponses => EndpointKind::Responses,
            Self::OpenAiChatCompletions => EndpointKind::ChatCompletions,
            Self::AnthropicMessages => EndpointKind::Messages,
            Self::GeminiGenerateContent => EndpointKind::Messages,
            Self::OpenAiEmbeddings => EndpointKind::Embeddings,
            Self::OpenAiImageGenerations => EndpointKind::ImageGenerations,
            Self::OpenAiAudioTranscriptions => EndpointKind::AudioTranscriptions,
            Self::OpenAiAudioSpeech => EndpointKind::AudioSpeech,
            Self::OpenAiRerank => EndpointKind::Rerank,
        }
    }

    /// Returns the canonical endpoint family for parsed/emitted values.
    pub fn canonical_endpoint_kind(self) -> EndpointKind {
        match self {
            Self::OpenAiResponses
            | Self::OpenAiChatCompletions
            | Self::AnthropicMessages
            | Self::GeminiGenerateContent => EndpointKind::Responses,
            Self::OpenAiEmbeddings => EndpointKind::Embeddings,
            Self::OpenAiImageGenerations => EndpointKind::ImageGenerations,
            Self::OpenAiAudioTranscriptions => EndpointKind::AudioTranscriptions,
            Self::OpenAiAudioSpeech => EndpointKind::AudioSpeech,
            Self::OpenAiRerank => EndpointKind::Rerank,
        }
    }

    pub fn is_generation(self) -> bool {
        matches!(
            self,
            Self::OpenAiResponses
                | Self::OpenAiChatCompletions
                | Self::AnthropicMessages
                | Self::GeminiGenerateContent
        )
    }
}

impl From<ProviderProtocol> for WireFormat {
    fn from(value: ProviderProtocol) -> Self {
        match value {
            ProviderProtocol::OpenAiResponses => Self::OpenAiResponses,
            ProviderProtocol::OpenAiChatCompletions => Self::OpenAiChatCompletions,
            ProviderProtocol::ClaudeMessages => Self::AnthropicMessages,
            ProviderProtocol::GeminiGenerateContent => Self::GeminiGenerateContent,
        }
    }
}

impl TryFrom<WireFormat> for ProviderProtocol {
    type Error = &'static str;

    fn try_from(value: WireFormat) -> Result<Self, Self::Error> {
        match value {
            WireFormat::OpenAiResponses => Ok(Self::OpenAiResponses),
            WireFormat::OpenAiChatCompletions => Ok(Self::OpenAiChatCompletions),
            WireFormat::AnthropicMessages => Ok(Self::ClaudeMessages),
            WireFormat::GeminiGenerateContent => Ok(Self::GeminiGenerateContent),
            _ => Err("wire format is not a generation protocol"),
        }
    }
}

/// A canonical multi-endpoint request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "endpoint", rename_all = "snake_case")]
pub enum ApiRequest {
    Responses(LlmRequest),
    Embeddings(EmbeddingRequest),
    ImageGenerations(ImageGenerationRequest),
    AudioTranscriptions(AudioTranscriptionRequest),
    AudioSpeech(AudioSpeechRequest),
    Rerank(RerankRequest),
}

impl ApiRequest {
    pub fn canonical_endpoint_kind(&self) -> EndpointKind {
        match self {
            Self::Responses(_) => EndpointKind::Responses,
            Self::Embeddings(_) => EndpointKind::Embeddings,
            Self::ImageGenerations(_) => EndpointKind::ImageGenerations,
            Self::AudioTranscriptions(_) => EndpointKind::AudioTranscriptions,
            Self::AudioSpeech(_) => EndpointKind::AudioSpeech,
            Self::Rerank(_) => EndpointKind::Rerank,
        }
    }
}

impl From<LlmRequest> for ApiRequest {
    fn from(value: LlmRequest) -> Self {
        Self::Responses(value)
    }
}

impl TryFrom<ApiRequest> for LlmRequest {
    type Error = &'static str;

    fn try_from(value: ApiRequest) -> Result<Self, Self::Error> {
        match value {
            ApiRequest::Responses(request) => Ok(request),
            _ => Err("api request is not a generation request"),
        }
    }
}

/// A canonical multi-endpoint response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "endpoint", rename_all = "snake_case")]
pub enum ApiResponse {
    Responses(LlmResponse),
    Embeddings(EmbeddingResponse),
    ImageGenerations(ImageGenerationResponse),
    AudioTranscriptions(AudioTranscriptionResponse),
    AudioSpeech(AudioSpeechResponse),
    Rerank(RerankResponse),
}

impl ApiResponse {
    pub fn canonical_endpoint_kind(&self) -> EndpointKind {
        match self {
            Self::Responses(_) => EndpointKind::Responses,
            Self::Embeddings(_) => EndpointKind::Embeddings,
            Self::ImageGenerations(_) => EndpointKind::ImageGenerations,
            Self::AudioTranscriptions(_) => EndpointKind::AudioTranscriptions,
            Self::AudioSpeech(_) => EndpointKind::AudioSpeech,
            Self::Rerank(_) => EndpointKind::Rerank,
        }
    }
}

impl From<LlmResponse> for ApiResponse {
    fn from(value: LlmResponse) -> Self {
        Self::Responses(value)
    }
}

impl TryFrom<ApiResponse> for LlmResponse {
    type Error = &'static str;

    fn try_from(value: ApiResponse) -> Result<Self, Self::Error> {
        match value {
            ApiResponse::Responses(response) => Ok(response),
            _ => Err("api response is not a generation response"),
        }
    }
}

/// A transport request emitted for an upstream API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportRequest {
    pub method: HttpMethod,
    pub path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<String>,
    pub body: RequestBody,
}

impl TransportRequest {
    pub fn json_post(path: impl Into<String>, value: Value) -> Self {
        Self {
            method: HttpMethod::Post,
            path: path.into(),
            headers: BTreeMap::new(),
            accept: None,
            body: RequestBody::Json { value },
        }
    }
}

/// A transport response parsed from an upstream API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportResponse {
    pub status: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub body: ResponseBody,
}

impl TransportResponse {
    pub fn json(status: u16, value: Value) -> Self {
        Self {
            status,
            headers: BTreeMap::new(),
            content_type: Some("application/json".into()),
            body: ResponseBody::Json { value },
        }
    }
}

/// HTTP method marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Transport request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    Json {
        value: Value,
    },
    Multipart {
        fields: Vec<MultipartField>,
    },
    Text {
        text: String,
    },
    Binary {
        data_base64: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

/// Transport response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseBody {
    Json {
        value: Value,
    },
    Text {
        text: String,
    },
    Binary {
        data_base64: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

/// Multipart field used for file-oriented APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartField {
    pub name: String,
    pub value: MultipartValue,
}

/// Multipart field value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultipartValue {
    Text {
        value: String,
    },
    File {
        filename: String,
        data_base64: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

/// Conversion result with explicit bridge/loss metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionReport<T> {
    pub value: T,
    pub canonical_endpoint: EndpointKind,
    pub wire_format: WireFormat,
    #[serde(default)]
    pub bridged: bool,
    #[serde(default)]
    pub lossy: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loss_reasons: Vec<String>,
}

impl<T> ConversionReport<T> {
    pub fn native(value: T, canonical_endpoint: EndpointKind, wire_format: WireFormat) -> Self {
        Self {
            value,
            canonical_endpoint,
            wire_format,
            bridged: false,
            lossy: false,
            loss_reasons: Vec::new(),
        }
    }

    pub fn bridged(
        value: T,
        canonical_endpoint: EndpointKind,
        wire_format: WireFormat,
        loss_reasons: Vec<String>,
    ) -> Self {
        let lossy = !loss_reasons.is_empty();
        Self {
            value,
            canonical_endpoint,
            wire_format,
            bridged: true,
            lossy,
            loss_reasons,
        }
    }

    pub fn map<U>(self, value: U) -> ConversionReport<U> {
        ConversionReport {
            value,
            canonical_endpoint: self.canonical_endpoint,
            wire_format: self.wire_format,
            bridged: self.bridged,
            lossy: self.lossy,
            loss_reasons: self.loss_reasons,
        }
    }
}

/// Canonical embeddings request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: Vec<EmbeddingInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// A single embedding input item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbeddingInput {
    Text { text: String },
    Tokens { tokens: Vec<i32> },
}

/// Canonical embeddings response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub model: String,
    pub data: Vec<EmbeddingVector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<EmbeddingUsage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub index: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

/// Canonical image generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Canonical image generation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    pub data: Vec<GeneratedImage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// Canonical audio transcription request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTranscriptionRequest {
    pub model: String,
    pub audio: AudioInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timestamp_granularities: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Canonical audio speech request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSpeechRequest {
    pub model: String,
    pub input: String,
    pub voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Canonical audio transcription response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTranscriptionResponse {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<AudioSegment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<TranscribedWord>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Canonical audio speech response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSpeechResponse {
    pub data_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Audio input variants for transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AudioInput {
    Url {
        url: String,
    },
    File {
        filename: String,
        data_base64: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSegment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribedWord {
    pub word: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
}

/// Canonical rerank request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<RerankDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_documents: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Canonical rerank response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub model: String,
    pub results: Vec<RerankResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<RerankUsage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RerankDocument {
    Text { text: String },
    Json { value: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: u32,
    pub relevance_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}
