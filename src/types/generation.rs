use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::protocol::ProviderProtocol;

use super::{Message, PromptCacheUsage, ToolCallPart, ToolResultPart, VendorExtensions};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache: Option<PromptCacheUsage>,
}

impl TokenUsage {
    /// Returns the total number of tokens used.
    pub fn total(&self) -> u32 {
        self.total_tokens
            .unwrap_or(self.prompt_tokens + self.completion_tokens)
    }
}
