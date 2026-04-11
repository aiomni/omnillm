use std::env;
use std::time::Duration;

use futures_util::StreamExt;
use omnillm::{
    AuthScheme, CapabilitySet, EndpointProtocol, GatewayBuilder, KeyConfig, LlmRequest, Message,
    MessagePart, MessageRole, ProviderEndpoint, RequestItem, ResponseItem, ToolDefinition,
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

fn live_gateway() -> omnillm::Gateway {
    dotenvy::dotenv().ok();

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

    GatewayBuilder::new(endpoint)
        .add_key(KeyConfig::new(
            required_env("OMNILLM_RESPONSES_API_KEY"),
            "responses-live",
        ))
        .request_timeout(Duration::from_secs(180))
        .build()
        .expect("build gateway")
}

#[tokio::test]
#[ignore = "live generic runtime call; run explicitly with OMNILLM_RESPONSES_* configured"]
async fn responses_vision_demo() {
    let gateway = live_gateway();

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
        generation: omnillm::GenerationConfig {
            max_output_tokens: configured_max_output_tokens(),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    if configured_stream() {
        let mut stream = gateway
            .stream(request, CancellationToken::new())
            .await
            .expect("live stream should start");
        let mut content = String::new();
        let mut completed_content = None;

        while let Some(event) = stream.next().await {
            match event.expect("stream event should parse") {
                omnillm::LlmStreamEvent::TextDelta { delta } => content.push_str(&delta),
                omnillm::LlmStreamEvent::Completed { response } => {
                    completed_content = Some(response.content_text)
                }
                _ => {}
            }
        }

        assert!(
            !content.trim().is_empty()
                || completed_content
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty()),
            "expected streamed content"
        );
    } else {
        let response = gateway
            .call(request, CancellationToken::new())
            .await
            .expect("live request should succeed");

        assert!(
            !response.content_text.trim().is_empty(),
            "expected non-empty content"
        );
    }
}

#[tokio::test]
#[ignore = "live generic runtime call; run explicitly with OMNILLM_RESPONSES_* configured"]
async fn responses_function_tool_demo() {
    let gateway = live_gateway();

    let request = LlmRequest {
        model: required_env("OMNILLM_RESPONSES_TOOL_MODEL"),
        instructions: None,
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            env::var("OMNILLM_RESPONSES_TOOL_PROMPT")
                .unwrap_or_else(|_| "What is the weather like in Boston today?".into()),
        ))],
        messages: Vec::new(),
        capabilities: CapabilitySet {
            tools: vec![ToolDefinition {
                name: "get_current_weather".into(),
                description: Some("Get the current weather in a given location".into()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state, e.g. San Francisco, CA"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"]
                        }
                    },
                    "required": ["location", "unit"]
                }),
                strict: false,
                vendor_extensions: Default::default(),
            }],
            ..Default::default()
        },
        generation: omnillm::GenerationConfig {
            max_output_tokens: configured_max_output_tokens(),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway
        .call(request, CancellationToken::new())
        .await
        .expect("live request should succeed");

    let has_tool_call = response
        .output
        .iter()
        .any(|item| matches!(item, ResponseItem::ToolCall { .. }));
    let has_text = !response.content_text.trim().is_empty();

    assert!(
        has_tool_call || has_text,
        "expected either a tool call or non-empty text content, got: {:?}",
        response.output
    );
}
