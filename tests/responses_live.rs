use std::env;
use std::time::Duration;

use omnillm::{
    AuthScheme, CapabilitySet, GatewayBuilder, KeyConfig, LlmRequest, Message, MessagePart,
    MessageRole, ProviderEndpoint, ProviderProtocol, RequestItem, ResponseItem, ToolDefinition,
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

fn live_gateway() -> omnillm::Gateway {
    dotenvy::dotenv().ok();

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
#[ignore = "live generic Responses call; run explicitly with OMNILLM_RESPONSES_* configured"]
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
        generation: Default::default(),
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway
        .call(request, CancellationToken::new())
        .await
        .expect("live request should succeed");

    assert!(
        !response.content_text.trim().is_empty(),
        "expected non-empty content"
    );
}

#[tokio::test]
#[ignore = "live generic Responses call; run explicitly with OMNILLM_RESPONSES_* configured"]
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
        generation: Default::default(),
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
