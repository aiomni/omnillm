use omnillm::{
    GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessageRole,
    ProviderEndpoint, RequestItem,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;

    let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .add_key(
            KeyConfig::new(api_key, "openai-prod-1")
                .tpm_limit(90_000)
                .rpm_limit(500),
        )
        .budget_limit_usd(50.0)
        .build()?;

    let request = LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: Some("Answer in one short paragraph.".into()),
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Explain what OmniLLM does.",
        ))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(128),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway.call(request, CancellationToken::new()).await?;
    println!("model: {}", response.model);
    println!("tokens: {}", response.usage.total());
    println!("{}", response.content_text);

    Ok(())
}
