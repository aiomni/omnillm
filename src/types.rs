//! Public canonical request/response types used by [`crate::Gateway`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::ProviderProtocol;

/// Arbitrary provider-specific extension payload.
pub type VendorExtensions = BTreeMap<String, Value>;

/// A provider-neutral LLM request with a Responses-style semantic core.
///
/// `input` is the execution source of truth. `messages` is preserved as a
/// compatibility view for chat-style APIs and ergonomic construction.
///
/// # Example
///
/// ```
/// use omnillm::{
///     GenerationConfig, LlmRequest, Message, MessagePart, MessageRole, RequestItem,
/// };
///
/// let req = LlmRequest {
///     model: "gpt-4.1-mini".into(),
///     instructions: Some("Answer concisely".into()),
///     input: vec![RequestItem::from(Message::text(
///         MessageRole::User,
///         "Hello!",
///     ))],
///     messages: Vec::new(),
///     capabilities: Default::default(),
///     generation: GenerationConfig {
///         max_output_tokens: Some(256),
///         ..Default::default()
///     },
///     metadata: Default::default(),
///     vendor_extensions: Default::default(),
/// };
///
/// assert!(matches!(
///     req.normalized_input().first(),
///     Some(RequestItem::Message { .. })
/// ));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Model identifier, e.g. `"gpt-4.1-mini"` or `"claude-sonnet-4-5"`.
    pub model: String,
    /// High-level instructions for the model, equivalent to system/developer
    /// messages in chat-style APIs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Canonical input items used by protocol emitters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<RequestItem>,
    /// Compatibility chat-style view.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    /// Unified capability layer.
    #[serde(default, skip_serializing_if = "CapabilitySet::is_empty")]
    pub capabilities: CapabilitySet,
    /// Generation controls independent of any provider wire format.
    #[serde(default, skip_serializing_if = "GenerationConfig::is_default")]
    pub generation: GenerationConfig,
    /// Request metadata forwarded when the target protocol supports it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: VendorExtensions,
    /// Provider-specific request extensions preserved across round-trips.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl LlmRequest {
    /// Returns the canonical input items used for execution.
    pub fn normalized_input(&self) -> Vec<RequestItem> {
        if !self.input.is_empty() {
            return self.input.clone();
        }

        self.messages
            .iter()
            .cloned()
            .map(RequestItem::from)
            .collect()
    }

    /// Returns the compatibility message view.
    pub fn normalized_messages(&self) -> Vec<Message> {
        if !self.messages.is_empty() {
            return self.messages.clone();
        }

        self.input
            .iter()
            .filter_map(RequestItem::as_message)
            .cloned()
            .collect()
    }

    /// Returns an instructions string, folding system/developer messages in
    /// when explicit instructions are absent.
    pub fn normalized_instructions(&self) -> Option<String> {
        if self.instructions.is_some() {
            return self.instructions.clone();
        }

        let folded = self
            .normalized_messages()
            .into_iter()
            .filter(|message| matches!(message.role, MessageRole::System | MessageRole::Developer))
            .map(|message| message.plain_text())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        if folded.is_empty() {
            None
        } else {
            Some(folded)
        }
    }

    /// Estimate the number of prompt tokens before sending.
    pub fn estimated_prompt_tokens(&self) -> u32 {
        let mut chars = self
            .normalized_input()
            .iter()
            .map(RequestItem::estimated_chars)
            .sum::<usize>();

        if let Some(instructions) = self.normalized_instructions() {
            chars += instructions.len();
        }

        (chars / 4).max(1) as u32
    }

    /// Conservative total token estimate before sending (prompt + max output budget).
    ///
    /// The real count is only available in the provider response.
    pub fn estimated_tokens(&self) -> u32 {
        self.estimated_prompt_tokens() + self.generation.max_output_tokens.unwrap_or(1024)
    }
}

/// A canonical input item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestItem {
    /// Chat-style message input.
    Message { message: Message },
    /// Standalone tool result input.
    ToolResult {
        #[serde(flatten)]
        result: ToolResultPart,
    },
}

impl RequestItem {
    fn estimated_chars(&self) -> usize {
        match self {
            Self::Message { message } => message.estimated_chars(),
            Self::ToolResult { result } => result.output.to_string().len(),
        }
    }

    pub(crate) fn as_message(&self) -> Option<&Message> {
        match self {
            Self::Message { message } => Some(message),
            Self::ToolResult { .. } => None,
        }
    }
}

impl From<Message> for RequestItem {
    fn from(message: Message) -> Self {
        Self::Message { message }
    }
}

/// A single role-tagged message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message author role.
    pub role: MessageRole,
    /// Message parts.
    pub parts: Vec<MessagePart>,
    /// Provider-native JSON for this message, if preserved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_message: Option<String>,
    /// Provider-specific extensions for this message.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl Message {
    /// Convenience constructor for a single text-part message.
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![MessagePart::Text { text: text.into() }],
            raw_message: None,
            vendor_extensions: VendorExtensions::new(),
        }
    }

    /// Returns the concatenated text from all text-like parts.
    pub fn plain_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(MessagePart::plain_text)
            .collect::<Vec<_>>()
            .join("")
    }

    fn estimated_chars(&self) -> usize {
        self.parts.iter().map(MessagePart::estimated_chars).sum()
    }
}

/// Canonical chat roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    Developer,
    System,
    User,
    Assistant,
    Tool,
}

/// Canonical content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePart {
    Text {
        text: String,
    },
    ImageUrl {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    ImageBase64 {
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
    Audio {
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        transcript: Option<String>,
    },
    File {
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    Json {
        value: Value,
    },
    ToolCall {
        #[serde(flatten)]
        call: ToolCallPart,
    },
    ToolResult {
        #[serde(flatten)]
        result: ToolResultPart,
    },
    Reasoning {
        text: String,
    },
    Refusal {
        text: String,
    },
}

impl MessagePart {
    pub(crate) fn plain_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } | Self::Reasoning { text } | Self::Refusal { text } => {
                Some(text.as_str())
            }
            Self::Audio {
                transcript: Some(text),
                ..
            } => Some(text.as_str()),
            _ => None,
        }
    }

    fn estimated_chars(&self) -> usize {
        match self {
            Self::Text { text } => text.len(),
            Self::ImageUrl { url, .. } => url.len(),
            Self::ImageBase64 { data, .. } => data.len() / 8,
            Self::Audio {
                data, transcript, ..
            } => transcript
                .as_ref()
                .map_or(data.len() / 8, std::string::String::len),
            Self::File { data, filename, .. } => data.as_ref().map_or_else(
                || filename.as_ref().map_or(0, std::string::String::len),
                |d| d.len() / 8,
            ),
            Self::Json { value } => value.to_string().len(),
            Self::ToolCall { call } => call.arguments.to_string().len() + call.name.len(),
            Self::ToolResult { result } => result.output.to_string().len(),
            Self::Reasoning { text } | Self::Refusal { text } => text.len(),
        }
    }
}

/// A tool call emitted or requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPart {
    pub call_id: String,
    pub name: String,
    pub arguments: Value,
}

/// A tool result returned to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPart {
    pub call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub output: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

/// Unified capability layer for model generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<StructuredOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modalities: Vec<OutputModality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety: Option<SafetySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheSettings>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub builtin_tools: Vec<BuiltinTool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl CapabilitySet {
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
            && self.structured_output.is_none()
            && self.reasoning.is_none()
            && self.modalities.is_empty()
            && self.safety.is_none()
            && self.cache.is_none()
            && self.builtin_tools.is_empty()
            && self.vendor_extensions.is_empty()
    }
}

/// A callable custom tool/function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strict: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Provider built-in tools exposed as generic capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuiltinTool {
    WebSearch,
    FileSearch,
    CodeExecution,
    ComputerUse,
    UrlContext,
    Maps,
    Mcp {
        #[serde(skip_serializing_if = "Option::is_none")]
        server_label: Option<String>,
    },
    Vendor {
        name: String,
        #[serde(default, skip_serializing_if = "Value::is_null")]
        payload: Value,
    },
}

/// Structured output / schema constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub schema: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strict: bool,
}

/// Reasoning-related controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Desired output modalities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputModality {
    Text,
    Image,
    Audio,
    Json,
}

/// Safety / policy settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

/// Cache hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

fn default_true() -> bool {
    true
}

/// Provider-neutral generation controls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl GenerationConfig {
    pub fn is_default(&self) -> bool {
        self.max_output_tokens.is_none()
            && self.temperature.is_none()
            && self.top_p.is_none()
            && self.top_k.is_none()
            && self.stop_sequences.is_empty()
            && self.presence_penalty.is_none()
            && self.frequency_penalty.is_none()
            && self.seed.is_none()
            && self.vendor_extensions.is_empty()
    }
}

/// A provider-neutral response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<ResponseItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_text: String,
    pub usage: TokenUsage,
    pub model: String,
    pub provider_protocol: ProviderProtocol,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vendor_extensions: VendorExtensions,
}

impl LlmResponse {
    /// Constructs a minimal response from a single assistant message.
    pub fn from_message(
        provider_protocol: ProviderProtocol,
        model: impl Into<String>,
        message: Message,
        usage: TokenUsage,
    ) -> Self {
        let content_text = message.plain_text();
        Self {
            output: vec![ResponseItem::Message {
                message: message.clone(),
            }],
            messages: vec![message],
            content_text,
            usage,
            model: model.into(),
            provider_protocol,
            finish_reason: None,
            response_id: None,
            vendor_extensions: VendorExtensions::new(),
        }
    }
}

/// Canonical response items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseItem {
    Message {
        message: Message,
    },
    ToolCall {
        #[serde(flatten)]
        call: ToolCallPart,
    },
    ToolResult {
        #[serde(flatten)]
        result: ToolResultPart,
    },
    Reasoning {
        text: String,
    },
    Refusal {
        text: String,
    },
}

/// Common finish reasons.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    ContentFilter,
    Cancelled,
    Error,
    Other(String),
}

/// Token usage reported in a provider response.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

impl TokenUsage {
    /// Returns the total number of tokens used.
    pub fn total(&self) -> u32 {
        self.total_tokens
            .unwrap_or(self.prompt_tokens + self.completion_tokens)
    }
}

/// Provider-neutral streaming events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmStreamEvent {
    ResponseStarted {
        #[serde(skip_serializing_if = "Option::is_none")]
        response_id: Option<String>,
        model: String,
        provider_protocol: ProviderProtocol,
    },
    OutputItemAdded {
        item: ResponseItem,
    },
    ContentPartAdded {
        part: MessagePart,
    },
    TextDelta {
        delta: String,
    },
    ToolCallDelta {
        call_id: String,
        name: String,
        delta: String,
    },
    ToolResult {
        result: ToolResultPart,
    },
    ReasoningDelta {
        delta: String,
    },
    Usage {
        usage: TokenUsage,
    },
    Completed {
        response: LlmResponse,
    },
    Error {
        message: String,
    },
}
