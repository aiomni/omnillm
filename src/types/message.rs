use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VendorExtensions;

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

    pub(super) fn estimated_chars(&self) -> usize {
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
