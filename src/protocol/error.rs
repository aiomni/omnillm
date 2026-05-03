use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid shape: {0}")]
    InvalidShape(String),
    #[error("unsupported feature for target protocol: {0}")]
    UnsupportedFeature(String),
}
