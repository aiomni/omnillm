//! # omni-gateway
//!
//! A production-grade Rust library for provider-neutral LLM access with
//! multi-key load balancing, protocol conversion, per-key rate limiting, and
//! lock-free cost tracking.
//!
//! ## Quick Start
//!
//! ```no_run
//! use omni_gateway::{
//!     GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessageRole,
//!     ProviderEndpoint, RequestItem,
//! };
//! use tokio_util::sync::CancellationToken;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
//!     .add_key(KeyConfig::new("sk-key-1", "prod-1").tpm_limit(90_000).rpm_limit(500))
//!     .budget_limit_usd(50.0)
//!     .build()?;
//!
//! let req = LlmRequest {
//!     model: "gpt-4.1-mini".into(),
//!     instructions: Some("Answer concisely".into()),
//!     input: vec![RequestItem::from(Message::text(MessageRole::User, "Hello!"))],
//!     messages: Vec::new(),
//!     capabilities: Default::default(),
//!     generation: GenerationConfig {
//!         max_output_tokens: Some(256),
//!         ..Default::default()
//!     },
//!     metadata: Default::default(),
//!     vendor_extensions: Default::default(),
//! };
//!
//! let resp = gateway.call(req, CancellationToken::new()).await?;
//! println!("{}", resp.content_text);
//! # Ok(())
//! # }
//! ```

pub mod budget;
pub mod config;
pub mod error;
pub mod key;
pub mod protocol;
pub mod types;

pub(crate) mod dispatcher;
pub(crate) mod limiter;
pub(crate) mod pricing;

mod gateway;

pub use budget::tracker::BudgetTracker;
pub use config::{GatewayConfig, KeyConfig, PoolConfig};
pub use error::{GatewayError, ProviderError};
pub use gateway::{Gateway, GatewayBuilder, GatewayStream};
pub use key::lease::KeyLease;
pub use key::pool::KeyStatus;
pub use protocol::{
    emit_error, emit_request, emit_response, emit_stream_event, parse_error, parse_request,
    parse_response, parse_stream_event, transcode_error, transcode_request, transcode_response,
    transcode_stream_event, AuthScheme, ProtocolError, ProviderEndpoint, ProviderProtocol,
    ProviderStreamFrame,
};
pub use types::{
    BuiltinTool, CacheSettings, CapabilitySet, FinishReason, GenerationConfig, LlmRequest,
    LlmResponse, LlmStreamEvent, Message, MessagePart, MessageRole, OutputModality,
    ReasoningCapability, RequestItem, ResponseItem, SafetySettings, StructuredOutputConfig,
    TokenUsage, ToolCallPart, ToolDefinition, ToolResultPart, VendorExtensions,
};
