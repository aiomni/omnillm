use serde::{Deserialize, Serialize};

/// A raw SSE frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStreamFrame {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub data: String,
}
