//! Generic live runtime gateway demo.
//!
//! Run with:
//! ```sh
//! cargo run --example responses_live_demo
//! ```

use std::env;
use std::io::{self, Write};
use std::time::Duration;

use futures_util::StreamExt;
use omnillm::{
    AuthScheme, EndpointProtocol, GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest,
    LlmStreamEvent, Message, MessagePart, MessageRole, ProviderEndpoint, RequestItem,
};
use tokio_util::sync::CancellationToken;

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name} in the environment or .env"))
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn configured_protocol() -> EndpointProtocol {
    env::var("OMNILLM_RESPONSES_PROTOCOL")
        .unwrap_or_else(|_| "openai_responses".into())
        .parse()
        .unwrap_or_else(|error| panic!("{error}"))
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

fn configured_stream() -> bool {
    matches!(
        env::var("OMNILLM_RESPONSES_STREAM")
            .unwrap_or_else(|_| "false".into())
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn configured_max_output_tokens() -> Option<u32> {
    optional_env("OMNILLM_RESPONSES_MAX_OUTPUT_TOKENS").map(|value| {
        value.parse().unwrap_or_else(|_| {
            panic!("OMNILLM_RESPONSES_MAX_OUTPUT_TOKENS must be a positive integer")
        })
    })
}

fn configured_endpoint() -> ProviderEndpoint {
    let mut endpoint = ProviderEndpoint::new(
        configured_protocol(),
        required_env("OMNILLM_RESPONSES_BASE_URL"),
    )
    .with_auth(configured_auth_scheme());

    if let (Some(name), Some(value)) = (
        optional_env("OMNILLM_RESPONSES_EXTRA_HEADER_NAME"),
        optional_env("OMNILLM_RESPONSES_EXTRA_HEADER_VALUE"),
    ) {
        endpoint = endpoint.with_default_header(name, value);
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
        generation: GenerationConfig {
            max_output_tokens: configured_max_output_tokens(),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    if configured_stream() {
        let mut stream = gateway.stream(request, CancellationToken::new()).await?;
        let mut completed = None;
        let mut saw_text = false;

        println!("content:");
        while let Some(event) = stream.next().await {
            match event? {
                LlmStreamEvent::TextDelta { delta } => {
                    print!("{delta}");
                    io::stdout().flush()?;
                    saw_text = true;
                }
                LlmStreamEvent::Completed { response } => completed = Some(response),
                _ => {}
            }
        }

        if saw_text {
            println!();
        }
        if let Some(response) = completed {
            println!("model: {}", response.model);
            println!("usage: {}", response.usage.total());
        }
    } else {
        let response = gateway.call(request, CancellationToken::new()).await?;

        println!("model: {}", response.model);
        println!("usage: {}", response.usage.total());
        println!("content:\n{}", response.content_text);
    }

    Ok(())
}
