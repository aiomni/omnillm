//! Generic live Responses demo.
//!
//! Run with:
//! ```sh
//! cargo run --example responses_live_demo
//! ```

use std::env;
use std::time::Duration;

use omnillm::{
    AuthScheme, GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessagePart,
    MessageRole, ProviderEndpoint, ProviderProtocol, RequestItem,
};
use tokio_util::sync::CancellationToken;

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name} in the environment or .env"))
}

fn configured_auth_scheme() -> AuthScheme {
    match env::var("OMNILLM_RESPONSES_AUTH_SCHEME")
        .unwrap_or_else(|_| "bearer".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "bearer" => AuthScheme::Bearer,
        "query" => AuthScheme::Query {
            name: required_env("OMNILLM_RESPONSES_AUTH_NAME"),
        },
        "header" => AuthScheme::Header {
            name: required_env("OMNILLM_RESPONSES_AUTH_NAME"),
        },
        other => panic!(
            "unsupported OMNILLM_RESPONSES_AUTH_SCHEME={other}; expected bearer, query, or header"
        ),
    }
}

fn configured_endpoint() -> ProviderEndpoint {
    let mut endpoint = ProviderEndpoint::new(
        ProviderProtocol::OpenAiResponses,
        required_env("OMNILLM_RESPONSES_BASE_URL"),
    )
    .with_auth(configured_auth_scheme());

    if let (Ok(name), Ok(value)) = (
        env::var("OMNILLM_RESPONSES_EXTRA_HEADER_NAME"),
        env::var("OMNILLM_RESPONSES_EXTRA_HEADER_VALUE"),
    ) {
        if !name.is_empty() {
            endpoint = endpoint.with_default_header(name, value);
        }
    }

    endpoint
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let gateway = GatewayBuilder::new(configured_endpoint())
        .add_key(KeyConfig::new(
            required_env("OMNILLM_RESPONSES_API_KEY"),
            "responses-live",
        ))
        .request_timeout(Duration::from_secs(180))
        .build()?;

    let request = LlmRequest {
        model: required_env("OMNILLM_RESPONSES_VISION_MODEL"),
        instructions: None,
        input: vec![RequestItem::from(Message {
            role: MessageRole::User,
            parts: vec![
                MessagePart::Text {
                    text: env::var("OMNILLM_RESPONSES_VISION_PROMPT")
                        .unwrap_or_else(|_| "what is in this image?".into()),
                },
                MessagePart::ImageUrl {
                    url: required_env("OMNILLM_RESPONSES_IMAGE_URL"),
                    detail: None,
                },
            ],
            raw_message: None,
            vendor_extensions: Default::default(),
        })],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig::default(),
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway.call(request, CancellationToken::new()).await?;

    println!("model: {}", response.model);
    println!("usage: {}", response.usage.total());
    println!("content:\n{}", response.content_text);

    Ok(())
}
