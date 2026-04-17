use std::io;
use std::time::Duration;

use futures_util::StreamExt;
use omnillm::{
    emit_stream_event, AuthScheme, GatewayBuilder, GatewayError, GenerationConfig, KeyConfig,
    LlmRequest, LlmStreamEvent, Message, MessageRole, ProviderEndpoint, ProviderProtocol,
    ProviderStreamFrame, RequestItem,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn stream_synthesizes_usage_when_chat_stream_ends_without_usage() {
    let request = test_request();
    let estimated_prompt_tokens = request.estimated_prompt_tokens();
    let text_delta = "abcdefgh";
    let completion_tokens = (text_delta.len() / 4) as u32;

    let body = sse_body(&[
        stream_frame(
            ProviderProtocol::OpenAiChatCompletions,
            LlmStreamEvent::ResponseStarted {
                response_id: Some("resp-chat".into()),
                model: request.model.clone(),
                provider_protocol: ProviderProtocol::OpenAiChatCompletions,
            },
        ),
        stream_frame(
            ProviderProtocol::OpenAiChatCompletions,
            LlmStreamEvent::TextDelta {
                delta: text_delta.into(),
            },
        ),
        ProviderStreamFrame {
            event: None,
            data: "[DONE]".into(),
        },
    ]);

    let (base_url, server) = spawn_server(ServerMode::Ok { body }).await;
    let gateway = test_gateway(ProviderProtocol::OpenAiChatCompletions, base_url);

    let mut stream = gateway
        .stream(request, CancellationToken::new())
        .await
        .expect("stream should start");

    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("chat stream should complete cleanly"));
    }

    let completed = events
        .into_iter()
        .find_map(|event| match event {
            LlmStreamEvent::Completed { response } => Some(response),
            _ => None,
        })
        .expect("gateway should synthesize a completed response");

    assert_eq!(completed.content_text, text_delta);
    assert_eq!(completed.usage.prompt_tokens, estimated_prompt_tokens);
    assert_eq!(completed.usage.completion_tokens, completion_tokens);
    assert!(
        (gateway.budget_used_usd()
            - expected_gpt4o_cost_usd(estimated_prompt_tokens, completion_tokens))
        .abs()
            < 1e-12
    );

    server
        .await
        .expect("server task should finish")
        .expect("server should succeed");
}

#[tokio::test]
async fn stream_preserves_first_chat_content_when_role_and_content_share_frame() {
    let request = test_request();
    let body = sse_body(&[
        ProviderStreamFrame {
            event: None,
            data: json!({
                "id": "resp-chat",
                "model": request.model.clone(),
                "choices": [{
                    "index": 0,
                    "delta": {
                        "role": "assistant",
                        "content": "hello"
                    }
                }]
            })
            .to_string(),
        },
        ProviderStreamFrame {
            event: None,
            data: "[DONE]".into(),
        },
    ]);

    let (base_url, server) = spawn_server(ServerMode::Ok { body }).await;
    let gateway = test_gateway(ProviderProtocol::OpenAiChatCompletions, base_url);

    let mut stream = gateway
        .stream(request, CancellationToken::new())
        .await
        .expect("stream should start");

    let mut streamed = String::new();
    let mut completed = None;
    while let Some(item) = stream.next().await {
        match item.expect("chat stream should complete cleanly") {
            LlmStreamEvent::TextDelta { delta } => streamed.push_str(&delta),
            LlmStreamEvent::Completed { response } => completed = Some(response),
            _ => {}
        }
    }

    assert_eq!(streamed, "hello");
    assert_eq!(
        completed
            .expect("gateway should synthesize a completed response")
            .content_text,
        "hello"
    );

    server
        .await
        .expect("server task should finish")
        .expect("server should succeed");
}

#[tokio::test]
async fn stream_error_without_usage_settles_budget_from_partial_output() {
    let request = test_request();
    let estimated_prompt_tokens = request.estimated_prompt_tokens();
    let text_delta = "done";
    let tool_name = "lookup_weather";
    let tool_delta_a = "{\"loc";
    let tool_delta_b = "ation\"}";
    let reasoning_delta = "reason";
    let generated_chars = text_delta.len()
        + tool_name.len()
        + tool_delta_a.len()
        + tool_delta_b.len()
        + reasoning_delta.len();
    let completion_tokens = (generated_chars / 4) as u32;

    let body = sse_body(&[
        stream_frame(
            ProviderProtocol::OpenAiResponses,
            LlmStreamEvent::ResponseStarted {
                response_id: Some("resp-responses".into()),
                model: request.model.clone(),
                provider_protocol: ProviderProtocol::OpenAiResponses,
            },
        ),
        stream_frame(
            ProviderProtocol::OpenAiResponses,
            LlmStreamEvent::TextDelta {
                delta: text_delta.into(),
            },
        ),
        stream_frame(
            ProviderProtocol::OpenAiResponses,
            LlmStreamEvent::ToolCallDelta {
                call_id: "call-1".into(),
                name: tool_name.into(),
                delta: tool_delta_a.into(),
            },
        ),
        stream_frame(
            ProviderProtocol::OpenAiResponses,
            LlmStreamEvent::ToolCallDelta {
                call_id: "call-1".into(),
                name: tool_name.into(),
                delta: tool_delta_b.into(),
            },
        ),
        stream_frame(
            ProviderProtocol::OpenAiResponses,
            LlmStreamEvent::ReasoningDelta {
                delta: reasoning_delta.into(),
            },
        ),
    ]);

    let (base_url, server) = spawn_server(ServerMode::TruncatedBody {
        body,
        declared_extra_bytes: 32,
    })
    .await;
    let gateway = test_gateway(ProviderProtocol::OpenAiResponses, base_url);

    let mut stream = gateway
        .stream(request, CancellationToken::new())
        .await
        .expect("stream should start");

    let mut terminal_error = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(error) => {
                terminal_error = Some(error);
                break;
            }
        }
    }

    assert!(
        matches!(terminal_error, Some(GatewayError::Provider(_))),
        "expected provider error, got {terminal_error:?}"
    );
    assert!(
        (gateway.budget_used_usd()
            - expected_gpt4o_cost_usd(estimated_prompt_tokens, completion_tokens))
        .abs()
            < 1e-12
    );

    server
        .await
        .expect("server task should finish")
        .expect("server should succeed");
}

fn test_request() -> LlmRequest {
    LlmRequest {
        model: "gpt-4o".into(),
        instructions: Some("Answer with a short payload.".into()),
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Return structured output.",
        ))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(32),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    }
}

fn test_gateway(protocol: ProviderProtocol, base_url: String) -> omnillm::Gateway {
    GatewayBuilder::new(
        ProviderEndpoint::new(protocol, base_url).with_auth(AuthScheme::Header {
            name: "x-test-key".into(),
        }),
    )
    .add_key(KeyConfig::new("test-key", "local-test"))
    .budget_limit_usd(1.0)
    .request_timeout(Duration::from_secs(5))
    .build()
    .expect("gateway should build")
}

fn expected_gpt4o_cost_usd(prompt_tokens: u32, completion_tokens: u32) -> f64 {
    let prompt_micro = (prompt_tokens as u64 * 5_000) / 1_000;
    let completion_micro = (completion_tokens as u64 * 15_000) / 1_000;
    (prompt_micro + completion_micro) as f64 / 1_000_000.0
}

fn stream_frame(protocol: ProviderProtocol, event: LlmStreamEvent) -> ProviderStreamFrame {
    emit_stream_event(protocol, &event)
        .expect("stream event should encode")
        .expect("event should map to an SSE frame")
}

fn sse_body(frames: &[ProviderStreamFrame]) -> String {
    frames.iter().map(sse_frame_text).collect()
}

fn sse_frame_text(frame: &ProviderStreamFrame) -> String {
    let mut text = String::new();
    if let Some(event) = &frame.event {
        text.push_str("event: ");
        text.push_str(event);
        text.push('\n');
    }
    if frame.data.is_empty() {
        text.push_str("data:\n\n");
        return text;
    }
    for line in frame.data.lines() {
        text.push_str("data: ");
        text.push_str(line);
        text.push('\n');
    }
    text.push('\n');
    text
}

enum ServerMode {
    Ok {
        body: String,
    },
    TruncatedBody {
        body: String,
        declared_extra_bytes: usize,
    },
}

async fn spawn_server(mode: ServerMode) -> (String, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let addr = listener
        .local_addr()
        .expect("listener should have local addr");
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        read_http_request(&mut stream).await?;

        match mode {
            ServerMode::Ok { body } => {
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.as_bytes().len(),
                    body,
                );
                stream.write_all(response.as_bytes()).await?;
            }
            ServerMode::TruncatedBody {
                body,
                declared_extra_bytes,
            } => {
                let response_headers = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.as_bytes().len() + declared_extra_bytes,
                );
                stream.write_all(response_headers.as_bytes()).await?;
                stream.write_all(body.as_bytes()).await?;
            }
        }

        stream.flush().await?;
        Ok(())
    });

    (format!("http://{}", addr), handle)
}

async fn read_http_request(stream: &mut TcpStream) -> io::Result<()> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = 0;

    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..read]);

        if header_end.is_none() {
            if let Some(index) = find_bytes(&buffer, b"\r\n\r\n") {
                let end = index + 4;
                header_end = Some(end);
                let headers = String::from_utf8_lossy(&buffer[..end]);
                content_length = parse_content_length(&headers);
            }
        }

        if let Some(end) = header_end {
            if buffer.len() >= end + content_length {
                return Ok(());
            }
        }
    }
}

fn parse_content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
