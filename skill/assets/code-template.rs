use std::time::Duration;

use omnillm::{
    AuthScheme, GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessageRole,
    ProviderEndpoint, ProviderProtocol, RequestItem,
};
use tokio_util::sync::CancellationToken;

fn configured_endpoint() -> ProviderEndpoint {
    // Replace this with a built-in helper such as
    // `ProviderEndpoint::openai_responses()` when you do not need a custom host.
    ProviderEndpoint::new(
        ProviderProtocol::OpenAiResponses,
        std::env::var("OMNILLM_BASE_URL").expect("set OMNILLM_BASE_URL"),
    )
    .with_auth(AuthScheme::Bearer)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = GatewayBuilder::new(configured_endpoint())
        .add_key(
            KeyConfig::new(
                std::env::var("OMNILLM_API_KEY").expect("set OMNILLM_API_KEY"),
                "primary",
            )
            .tpm_limit(90_000)
            .rpm_limit(500),
        )
        .request_timeout(Duration::from_secs(120))
        .budget_limit_usd(25.0)
        .build()?;

    let request = LlmRequest {
        model: std::env::var("OMNILLM_MODEL").expect("set OMNILLM_MODEL"),
        instructions: Some("Replace with your system guidance.".into()),
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Replace with your prompt.",
        ))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(256),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway.call(request, CancellationToken::new()).await?;
    println!("{}", response.content_text);

    Ok(())
}
