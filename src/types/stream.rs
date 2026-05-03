use serde::{Deserialize, Serialize};

use crate::protocol::ProviderProtocol;

use super::{LlmResponse, MessagePart, ResponseItem, TokenUsage, ToolResultPart};

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
