use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    CapabilitySet, GenerationConfig, Message, MessageRole, ToolResultPart, VendorExtensions,
};

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
