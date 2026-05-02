//! Demonstrates P1 primitive metadata requests without canonical conversion.
//!
//! The example runs only when one of the provider API key environment variables
//! is set. It is safe to compile without credentials.

use omnillm::{
    GatewayBuilder, KeyConfig, PrimitiveEndpointKind, PrimitiveProviderEndpoint,
    PrimitiveProviderKind, PrimitiveRequest, ProviderEndpoint, ProviderPrimitiveWireFormat,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
            .primitive_endpoint(PrimitiveProviderEndpoint::openai())
            .add_key(KeyConfig::new(api_key, "openai-p1"))
            .budget_limit_usd(1.0)
            .build()?;
        let response = gateway
            .primitive_call(
                PrimitiveRequest::get(
                    PrimitiveProviderKind::OpenAi,
                    PrimitiveEndpointKind::Models,
                    ProviderPrimitiveWireFormat::OpenAiModels,
                    Option::<String>::None,
                ),
                CancellationToken::new(),
            )
            .await?;
        println!("openai models status={}", response.status);
    }

    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
            .primitive_endpoint(PrimitiveProviderEndpoint::anthropic())
            .add_key(KeyConfig::new(api_key, "anthropic-p1"))
            .budget_limit_usd(1.0)
            .build()?;
        let response = gateway
            .primitive_call(
                PrimitiveRequest::get(
                    PrimitiveProviderKind::Anthropic,
                    PrimitiveEndpointKind::Models,
                    ProviderPrimitiveWireFormat::AnthropicModels,
                    Option::<String>::None,
                ),
                CancellationToken::new(),
            )
            .await?;
        println!("anthropic models status={}", response.status);
    }

    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
            .primitive_endpoint(PrimitiveProviderEndpoint::gemini())
            .add_key(KeyConfig::new(api_key, "gemini-p1"))
            .budget_limit_usd(1.0)
            .build()?;
        let response = gateway
            .primitive_call(
                PrimitiveRequest::get(
                    PrimitiveProviderKind::Gemini,
                    PrimitiveEndpointKind::Models,
                    ProviderPrimitiveWireFormat::GeminiModels,
                    Option::<String>::None,
                ),
                CancellationToken::new(),
            )
            .await?;
        println!("gemini models status={}", response.status);
    }

    Ok(())
}
