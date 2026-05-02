//! Demonstrates provider primitive request construction without canonical conversion.
//!
//! Run with:
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run --example primitive_protocol_demo
//! ```

use omnillm::{
    embedded_primitive_provider_registry, GatewayBuilder, KeyConfig, PrimitiveEndpointKind,
    PrimitiveProviderEndpoint, PrimitiveProviderKind, PrimitiveRequest, PrimitiveStreamMode,
    ProviderEndpoint, ProviderPrimitiveWireFormat,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = embedded_primitive_provider_registry();
    println!(
        "OpenAI primitive Responses supported: {}",
        registry.supports_wire_format(
            PrimitiveProviderKind::OpenAi,
            ProviderPrimitiveWireFormat::OpenAiResponses,
            PrimitiveStreamMode::None,
        )
    );

    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        println!("Set OPENAI_API_KEY to run the live primitive call.");
        return Ok(());
    };

    let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .primitive_endpoint(PrimitiveProviderEndpoint::openai())
        .add_key(KeyConfig::new(api_key, "openai-primitive"))
        .budget_limit_usd(5.0)
        .build()?;

    let response = gateway
        .primitive_call(
            PrimitiveRequest::json(
                PrimitiveProviderKind::OpenAi,
                PrimitiveEndpointKind::Responses,
                ProviderPrimitiveWireFormat::OpenAiResponses,
                "gpt-4o-mini",
                json!({"model":"gpt-4o-mini","input":"Say hello from primitive mode."}),
            ),
            CancellationToken::new(),
        )
        .await?;

    println!("status={} usage={:?}", response.status, response.usage);
    println!("body={:?}", response.body);
    Ok(())
}
