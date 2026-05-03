//! Provider protocol definitions and canonical/raw conversion helpers.

use serde_json::{json, Value};

use crate::error::ProviderError;
use crate::types::{LlmRequest, LlmResponse, LlmStreamEvent};

mod claude;
mod common;
mod endpoint;
mod error;
mod gemini;
mod openai;
mod stream_frame;

pub use endpoint::{AuthScheme, EndpointProtocol, ProviderEndpoint, ProviderProtocol};
pub use error::ProtocolError;
pub use stream_frame::ProviderStreamFrame;

use claude::{
    emit_claude_request, emit_claude_response, emit_claude_stream_event, parse_claude_error,
    parse_claude_request, parse_claude_response, parse_claude_stream_event,
};
use gemini::{
    emit_gemini_request, emit_gemini_response, emit_gemini_stream_event,
    emit_gemini_transport_request, parse_gemini_error, parse_gemini_request, parse_gemini_response,
    parse_gemini_stream_event,
};
use openai::{
    emit_openai_chat_request, emit_openai_chat_response, emit_openai_chat_stream_event,
    emit_openai_responses_request, emit_openai_responses_response,
    emit_openai_responses_stream_event, parse_openai_chat_request, parse_openai_chat_response,
    parse_openai_chat_stream_events, parse_openai_error, parse_openai_responses_request,
    parse_openai_responses_response, parse_openai_responses_stream_event,
};

pub fn parse_request(
    protocol: ProviderProtocol,
    raw_json: &str,
) -> Result<LlmRequest, ProtocolError> {
    let body: Value = serde_json::from_str(raw_json)?;
    match protocol {
        ProviderProtocol::OpenAiResponses => parse_openai_responses_request(&body),
        ProviderProtocol::OpenAiChatCompletions => parse_openai_chat_request(&body),
        ProviderProtocol::ClaudeMessages => parse_claude_request(&body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_request(&body),
    }
}

pub fn emit_request(
    protocol: ProviderProtocol,
    request: &LlmRequest,
) -> Result<String, ProtocolError> {
    serde_json::to_string(&emit_request_value(protocol, request, false, false)?)
        .map_err(ProtocolError::from)
}

pub fn transcode_request(
    from: ProviderProtocol,
    to: ProviderProtocol,
    raw_json: &str,
) -> Result<String, ProtocolError> {
    let request = parse_request(from, raw_json)?;
    emit_request(to, &request)
}

pub fn parse_response(
    protocol: ProviderProtocol,
    raw_json: &str,
) -> Result<LlmResponse, ProtocolError> {
    let body: Value = serde_json::from_str(raw_json)?;
    match protocol {
        ProviderProtocol::OpenAiResponses => parse_openai_responses_response(&body),
        ProviderProtocol::OpenAiChatCompletions => parse_openai_chat_response(&body),
        ProviderProtocol::ClaudeMessages => parse_claude_response(&body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_response(&body),
    }
}

pub fn emit_response(
    protocol: ProviderProtocol,
    response: &LlmResponse,
) -> Result<String, ProtocolError> {
    let body = match protocol {
        ProviderProtocol::OpenAiResponses => emit_openai_responses_response(response)?,
        ProviderProtocol::OpenAiChatCompletions => emit_openai_chat_response(response)?,
        ProviderProtocol::ClaudeMessages => emit_claude_response(response)?,
        ProviderProtocol::GeminiGenerateContent => emit_gemini_response(response)?,
    };
    serde_json::to_string(&body).map_err(ProtocolError::from)
}

pub fn transcode_response(
    from: ProviderProtocol,
    to: ProviderProtocol,
    raw_json: &str,
) -> Result<String, ProtocolError> {
    let response = parse_response(from, raw_json)?;
    emit_response(to, &response)
}

pub fn parse_error(
    protocol: ProviderProtocol,
    status: Option<u16>,
    raw_json: &str,
) -> Result<ProviderError, ProtocolError> {
    let body: Value = serde_json::from_str(raw_json)?;
    Ok(match protocol {
        ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
            parse_openai_error(protocol, status, &body)
        }
        ProviderProtocol::ClaudeMessages => parse_claude_error(status, &body),
        ProviderProtocol::GeminiGenerateContent => parse_gemini_error(status, &body),
    })
}

pub fn emit_error(
    protocol: ProviderProtocol,
    error: &ProviderError,
) -> Result<String, ProtocolError> {
    let body = match protocol {
        ProviderProtocol::OpenAiResponses | ProviderProtocol::OpenAiChatCompletions => {
            json!({
                "error": {
                    "message": error.message,
                    "type": error.code.clone().unwrap_or_else(|| "invalid_request_error".into()),
                    "code": error.code,
                }
            })
        }
        ProviderProtocol::ClaudeMessages => json!({
            "type": "error",
            "error": {
                "type": error.code.clone().unwrap_or_else(|| "api_error".into()),
                "message": error.message,
            }
        }),
        ProviderProtocol::GeminiGenerateContent => json!({
            "error": {
                "code": error.status.unwrap_or(500),
                "status": error.code.clone().unwrap_or_else(|| "INTERNAL".into()),
                "message": error.message,
            }
        }),
    };
    serde_json::to_string(&body).map_err(ProtocolError::from)
}

pub fn transcode_error(
    from: ProviderProtocol,
    to: ProviderProtocol,
    status: Option<u16>,
    raw_json: &str,
) -> Result<String, ProtocolError> {
    let error = parse_error(from, status, raw_json)?;
    emit_error(to, &error)
}

pub(crate) fn parse_stream_events(
    protocol: ProviderProtocol,
    frame: &ProviderStreamFrame,
) -> Result<Vec<LlmStreamEvent>, ProtocolError> {
    if frame.data.trim() == "[DONE]" {
        return Ok(Vec::new());
    }

    let body: Value = serde_json::from_str(&frame.data)?;
    match protocol {
        ProviderProtocol::OpenAiResponses => Ok(parse_openai_responses_stream_event(frame, &body)?
            .into_iter()
            .collect()),
        ProviderProtocol::OpenAiChatCompletions => parse_openai_chat_stream_events(&body),
        ProviderProtocol::ClaudeMessages => Ok(parse_claude_stream_event(frame, &body)?
            .into_iter()
            .collect()),
        ProviderProtocol::GeminiGenerateContent => {
            Ok(parse_gemini_stream_event(&body)?.into_iter().collect())
        }
    }
}

pub fn parse_stream_event(
    protocol: ProviderProtocol,
    frame: &ProviderStreamFrame,
) -> Result<Option<LlmStreamEvent>, ProtocolError> {
    Ok(select_primary_stream_event(parse_stream_events(
        protocol, frame,
    )?))
}

pub fn emit_stream_event(
    protocol: ProviderProtocol,
    event: &LlmStreamEvent,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    let frame = match protocol {
        ProviderProtocol::OpenAiResponses => emit_openai_responses_stream_event(event)?,
        ProviderProtocol::OpenAiChatCompletions => emit_openai_chat_stream_event(event)?,
        ProviderProtocol::ClaudeMessages => emit_claude_stream_event(event)?,
        ProviderProtocol::GeminiGenerateContent => emit_gemini_stream_event(event)?,
    };
    Ok(frame)
}

#[cfg(test)]
fn transcode_stream_events(
    from: ProviderProtocol,
    to: ProviderProtocol,
    frame: &ProviderStreamFrame,
) -> Result<Vec<ProviderStreamFrame>, ProtocolError> {
    let mut frames = Vec::new();
    for event in parse_stream_events(from, frame)? {
        if let Some(frame) = emit_stream_event(to, &event)? {
            frames.push(frame);
        }
    }
    Ok(frames)
}

pub fn transcode_stream_event(
    from: ProviderProtocol,
    to: ProviderProtocol,
    frame: &ProviderStreamFrame,
) -> Result<Option<ProviderStreamFrame>, ProtocolError> {
    match parse_stream_event(from, frame)? {
        Some(event) => emit_stream_event(to, &event),
        None => Ok(None),
    }
}

fn select_primary_stream_event(events: Vec<LlmStreamEvent>) -> Option<LlmStreamEvent> {
    let mut fallback = None;
    for event in events {
        if !matches!(event, LlmStreamEvent::ResponseStarted { .. }) {
            return Some(event);
        }
        if fallback.is_none() {
            fallback = Some(event);
        }
    }
    fallback
}

pub(crate) fn emit_request_with_mode(
    protocol: ProviderProtocol,
    request: &LlmRequest,
    stream: bool,
) -> Result<String, ProtocolError> {
    serde_json::to_string(&emit_request_value(protocol, request, stream, true)?)
        .map_err(ProtocolError::from)
}

pub(crate) fn take_sse_frames(buffer: &mut String) -> Vec<ProviderStreamFrame> {
    let normalized = buffer.replace("\r\n", "\n");
    *buffer = normalized;

    let mut frames = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let block = buffer[..idx].to_string();
        *buffer = buffer[idx + 2..].to_string();

        let mut event = None;
        let mut data_lines = Vec::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("event:") {
                event = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim_start().to_string());
            }
        }

        if !data_lines.is_empty() {
            frames.push(ProviderStreamFrame {
                event,
                data: data_lines.join("\n"),
            });
        }
    }

    frames
}

fn emit_request_value(
    protocol: ProviderProtocol,
    request: &LlmRequest,
    stream: bool,
    transport: bool,
) -> Result<Value, ProtocolError> {
    match protocol {
        ProviderProtocol::OpenAiResponses => emit_openai_responses_request(request, stream),
        ProviderProtocol::OpenAiChatCompletions => emit_openai_chat_request(request, stream),
        ProviderProtocol::ClaudeMessages => emit_claude_request(request, stream),
        ProviderProtocol::GeminiGenerateContent => {
            if transport {
                emit_gemini_transport_request(request)
            } else {
                emit_gemini_request(request)
            }
        }
    }
}

#[cfg(test)]
mod tests;
