use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Low-level upstream generation wire protocols used by the codec/transcoder
/// layer.
///
/// These names follow upstream endpoint families, not runtime configuration
/// presets. For example, `ClaudeMessages` refers to Anthropic's `/messages`
/// API, and `GeminiGenerateContent` refers to Gemini's `generateContent` API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProtocol {
    OpenAiResponses,
    OpenAiChatCompletions,
    ClaudeMessages,
    GeminiGenerateContent,
}

/// Runtime endpoint profiles used by [`ProviderEndpoint`].
///
/// Official variants derive request URLs from a base host/prefix. `Compat`
/// variants reuse the same wire protocol against a non-standard endpoint and
/// treat `base_url` as the final request URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointProtocol {
    OpenAiResponses,
    OpenAiChatCompletions,
    ClaudeMessages,
    GeminiGenerateContent,
    OpenAiResponsesCompat,
    OpenAiChatCompletionsCompat,
    ClaudeMessagesCompat,
    GeminiGenerateContentCompat,
}

impl EndpointProtocol {
    pub fn wire_protocol(self) -> ProviderProtocol {
        match self {
            Self::OpenAiResponses | Self::OpenAiResponsesCompat => {
                ProviderProtocol::OpenAiResponses
            }
            Self::OpenAiChatCompletions | Self::OpenAiChatCompletionsCompat => {
                ProviderProtocol::OpenAiChatCompletions
            }
            Self::ClaudeMessages | Self::ClaudeMessagesCompat => ProviderProtocol::ClaudeMessages,
            Self::GeminiGenerateContent | Self::GeminiGenerateContentCompat => {
                ProviderProtocol::GeminiGenerateContent
            }
        }
    }

    pub fn is_compat(self) -> bool {
        matches!(
            self,
            Self::OpenAiResponsesCompat
                | Self::OpenAiChatCompletionsCompat
                | Self::ClaudeMessagesCompat
                | Self::GeminiGenerateContentCompat
        )
    }
}

impl From<ProviderProtocol> for EndpointProtocol {
    fn from(value: ProviderProtocol) -> Self {
        match value {
            ProviderProtocol::OpenAiResponses => Self::OpenAiResponses,
            ProviderProtocol::OpenAiChatCompletions => Self::OpenAiChatCompletions,
            ProviderProtocol::ClaudeMessages => Self::ClaudeMessages,
            ProviderProtocol::GeminiGenerateContent => Self::GeminiGenerateContent,
        }
    }
}

impl FromStr for EndpointProtocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "responses" | "openai_responses" | "open_ai_responses" => Ok(Self::OpenAiResponses),
            "chat_completions" | "openai_chat_completions" | "open_ai_chat_completions" => {
                Ok(Self::OpenAiChatCompletions)
            }
            "claude_messages" | "anthropic_messages" => Ok(Self::ClaudeMessages),
            "gemini_generate_content" => Ok(Self::GeminiGenerateContent),
            "responses_compat" | "openai_responses_compat" | "open_ai_responses_compat" => {
                Ok(Self::OpenAiResponsesCompat)
            }
            "chat_completions_compat"
            | "openai_chat_completions_compat"
            | "open_ai_chat_completions_compat" => Ok(Self::OpenAiChatCompletionsCompat),
            "claude_messages_compat" | "anthropic_messages_compat" => {
                Ok(Self::ClaudeMessagesCompat)
            }
            "gemini_generate_content_compat" => Ok(Self::GeminiGenerateContentCompat),
            _ => Err(format!(
                "unsupported endpoint protocol `{value}`; expected one of: \
openai_responses, openai_chat_completions, claude_messages, gemini_generate_content, \
openai_responses_compat, openai_chat_completions_compat, claude_messages_compat, \
gemini_generate_content_compat"
            )),
        }
    }
}

/// Authentication strategy for an upstream provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthScheme {
    Bearer,
    Header { name: String },
    Query { name: String },
}

impl AuthScheme {
    pub fn default_for(protocol: EndpointProtocol) -> Self {
        match protocol.wire_protocol() {
            ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
                Self::Bearer
            }
            ProviderProtocol::ClaudeMessages => Self::Header {
                name: "x-api-key".into(),
            },
            ProviderProtocol::GeminiGenerateContent => Self::Header {
                name: "x-goog-api-key".into(),
            },
        }
    }
}

/// Target provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoint {
    pub protocol: EndpointProtocol,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthScheme>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub default_headers: BTreeMap<String, String>,
}

impl ProviderEndpoint {
    pub fn new(protocol: impl Into<EndpointProtocol>, base_url: impl Into<String>) -> Self {
        let protocol = protocol.into();
        let mut endpoint = Self {
            protocol,
            base_url: base_url.into(),
            auth: None,
            default_headers: BTreeMap::new(),
        };

        if matches!(protocol.wire_protocol(), ProviderProtocol::ClaudeMessages) {
            endpoint
                .default_headers
                .insert("anthropic-version".into(), "2023-06-01".into());
        }

        endpoint
    }

    pub fn openai_responses() -> Self {
        Self::new(
            EndpointProtocol::OpenAiResponses,
            "https://api.openai.com/v1",
        )
    }

    pub fn openai_chat_completions() -> Self {
        Self::new(
            EndpointProtocol::OpenAiChatCompletions,
            "https://api.openai.com/v1",
        )
    }

    pub fn claude_messages() -> Self {
        Self::new(
            EndpointProtocol::ClaudeMessages,
            "https://api.anthropic.com/v1",
        )
    }

    pub fn gemini_generate_content() -> Self {
        Self::new(
            EndpointProtocol::GeminiGenerateContent,
            "https://generativelanguage.googleapis.com",
        )
    }

    pub fn openai_responses_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::OpenAiResponsesCompat, base_url)
    }

    pub fn openai_chat_completions_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::OpenAiChatCompletionsCompat, base_url)
    }

    pub fn claude_messages_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::ClaudeMessagesCompat, base_url)
    }

    pub fn gemini_generate_content_compat(base_url: impl Into<String>) -> Self {
        Self::new(EndpointProtocol::GeminiGenerateContentCompat, base_url)
    }

    pub fn with_default_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    pub fn with_auth(mut self, auth: AuthScheme) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn auth_scheme(&self) -> AuthScheme {
        self.auth
            .clone()
            .unwrap_or_else(|| AuthScheme::default_for(self.protocol))
    }

    pub fn wire_protocol(&self) -> ProviderProtocol {
        self.protocol.wire_protocol()
    }

    pub(crate) fn request_url(&self, model: &str, stream: bool) -> String {
        if self.protocol.is_compat() {
            return self.base_url.trim().to_string();
        }

        let base = self.base_url.trim_end_matches('/');
        match self.protocol.wire_protocol() {
            ProviderProtocol::OpenAiResponses => {
                if base.ends_with("/responses") {
                    base.to_string()
                } else {
                    format!("{base}/responses")
                }
            }
            ProviderProtocol::OpenAiChatCompletions => {
                if base.ends_with("/chat/completions") {
                    base.to_string()
                } else {
                    format!("{base}/chat/completions")
                }
            }
            ProviderProtocol::ClaudeMessages => {
                if base.ends_with("/messages") {
                    base.to_string()
                } else {
                    format!("{base}/messages")
                }
            }
            ProviderProtocol::GeminiGenerateContent => {
                let prefix = if base.ends_with("/v1beta") {
                    base.to_string()
                } else {
                    format!("{base}/v1beta")
                };
                if stream {
                    format!("{prefix}/models/{model}:streamGenerateContent?alt=sse")
                } else {
                    format!("{prefix}/models/{model}:generateContent")
                }
            }
        }
    }
}
