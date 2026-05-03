use serde_json::{json, Map, Value};

use crate::types::{
    BuiltinTool, CapabilitySet, Message, MessagePart, MessageRole, StructuredOutputConfig,
    ToolCallPart, ToolDefinition, ToolResultPart, VendorExtensions,
};

use super::super::common::{parse_message_role, required_str};
use super::super::ProtocolError;

pub(in crate::protocol::gemini) fn parse_gemini_instruction(
    value: &Value,
) -> Result<String, ProtocolError> {
    let message = parse_gemini_content(value)?;
    Ok(message.plain_text())
}

pub(in crate::protocol::gemini) fn parse_gemini_capabilities(
    body: &Value,
) -> Result<CapabilitySet, ProtocolError> {
    let mut capabilities = CapabilitySet::default();
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            if let Some(decls) = tool.get("functionDeclarations").and_then(Value::as_array) {
                for decl in decls {
                    capabilities.tools.push(ToolDefinition {
                        name: required_str(decl, "name")?.to_string(),
                        description: decl
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        input_schema: decl
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                        strict: false,
                        vendor_extensions: VendorExtensions::new(),
                    });
                }
            }
            if tool.get("codeExecution").is_some() {
                capabilities.builtin_tools.push(BuiltinTool::CodeExecution);
            }
        }
    }
    if let Some(config) = body.get("generationConfig") {
        if let Some(schema) = config.get("responseSchema") {
            capabilities.structured_output = Some(StructuredOutputConfig {
                name: None,
                schema: schema.clone(),
                strict: false,
            });
        }
    }
    Ok(capabilities)
}

pub(in crate::protocol::gemini) fn emit_gemini_tools(
    capabilities: &CapabilitySet,
) -> Result<Vec<Value>, ProtocolError> {
    let mut tools = Vec::new();
    if !capabilities.tools.is_empty() {
        tools.push(json!({
            "functionDeclarations": capabilities.tools.iter().map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                })
            }).collect::<Vec<_>>()
        }));
    }
    for builtin in &capabilities.builtin_tools {
        match builtin {
            BuiltinTool::CodeExecution => tools.push(json!({ "codeExecution": {} })),
            other => {
                return Err(ProtocolError::UnsupportedFeature(format!(
                    "Gemini builtin tool {:?}",
                    other
                )))
            }
        }
    }
    Ok(tools)
}

pub(in crate::protocol::gemini) fn parse_gemini_content(
    value: &Value,
) -> Result<Message, ProtocolError> {
    let role = match value.get("role").and_then(Value::as_str).unwrap_or("user") {
        "model" => MessageRole::Assistant,
        "user" => MessageRole::User,
        "tool" => MessageRole::Tool,
        "system" => MessageRole::System,
        other => parse_message_role(other),
    };
    let parts = value
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolError::MissingField("parts".into()))?
        .iter()
        .map(|part| {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                return Ok(MessagePart::Text {
                    text: text.to_string(),
                });
            }
            if let Some(inline) = part.get("inlineData") {
                let media_type = inline
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let data = inline
                    .get("data")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if media_type.as_deref().unwrap_or("").starts_with("image/") {
                    return Ok(MessagePart::ImageBase64 { data, media_type });
                }
                if media_type.as_deref().unwrap_or("").starts_with("audio/") {
                    return Ok(MessagePart::Audio {
                        data,
                        media_type,
                        transcript: None,
                    });
                }
                return Ok(MessagePart::File {
                    file_id: None,
                    media_type,
                    data: Some(data),
                    filename: None,
                });
            }
            if let Some(file_data) = part.get("fileData") {
                return Ok(MessagePart::File {
                    file_id: file_data
                        .get("fileUri")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    media_type: file_data
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    data: None,
                    filename: None,
                });
            }
            if let Some(function_call) = part.get("functionCall") {
                return Ok(MessagePart::ToolCall {
                    call: ToolCallPart {
                        call_id: function_call
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        name: function_call
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        arguments: function_call
                            .get("args")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(Map::new())),
                    },
                });
            }
            if let Some(function_response) = part.get("functionResponse") {
                return Ok(MessagePart::ToolResult {
                    result: ToolResultPart {
                        call_id: function_response
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        name: function_response
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        output: function_response
                            .get("response")
                            .cloned()
                            .unwrap_or(Value::Null),
                        is_error: false,
                    },
                });
            }
            Ok(MessagePart::Json {
                value: part.clone(),
            })
        })
        .collect::<Result<Vec<_>, ProtocolError>>()?;
    Ok(Message {
        role,
        parts,
        raw_message: Some(value.to_string()),
        vendor_extensions: VendorExtensions::new(),
    })
}

pub(in crate::protocol::gemini) fn parse_gemini_candidate(
    value: &Value,
) -> Result<Message, ProtocolError> {
    parse_gemini_content(value.get("content").unwrap_or(value))
}

pub(in crate::protocol::gemini) fn gemini_content_json(
    message: Message,
) -> Result<Value, ProtocolError> {
    let role = match message.role {
        MessageRole::Assistant => "model",
        MessageRole::System => "system",
        MessageRole::Tool => "user",
        _ => "user",
    };
    Ok(json!({
        "role": role,
        "parts": gemini_parts(&message.parts)?,
    }))
}

pub(in crate::protocol::gemini) fn gemini_parts(
    parts: &[MessagePart],
) -> Result<Vec<Value>, ProtocolError> {
    parts
        .iter()
        .cloned()
        .map(|part| {
            Ok(match part {
                MessagePart::Text { text }
                | MessagePart::Reasoning { text }
                | MessagePart::Refusal { text } => json!({ "text": text }),
                MessagePart::ImageBase64 { data, media_type } => json!({
                    "inlineData": {
                        "mimeType": media_type.unwrap_or_else(|| "image/png".into()),
                        "data": data,
                    }
                }),
                MessagePart::ImageUrl { url, detail: _ } => json!({
                    "fileData": {
                        "fileUri": url,
                        "mimeType": "image/*",
                    }
                }),
                MessagePart::Audio { data, media_type, .. } => json!({
                    "inlineData": {
                        "mimeType": media_type.unwrap_or_else(|| "audio/wav".into()),
                        "data": data,
                    }
                }),
                MessagePart::File {
                    file_id,
                    media_type,
                    data,
                    filename: _,
                } => {
                    if let Some(data) = data {
                        json!({
                            "inlineData": {
                                "mimeType": media_type.unwrap_or_else(|| "application/octet-stream".into()),
                                "data": data,
                            }
                        })
                    } else {
                        json!({
                            "fileData": {
                                "fileUri": file_id.unwrap_or_default(),
                                "mimeType": media_type.unwrap_or_else(|| "application/octet-stream".into()),
                            }
                        })
                    }
                }
                MessagePart::ToolCall { call } => json!({
                    "functionCall": {
                        "id": call.call_id,
                        "name": call.name,
                        "args": call.arguments,
                    }
                }),
                MessagePart::ToolResult { result } => json!({
                    "functionResponse": {
                        "id": result.call_id,
                        "name": result.name.unwrap_or_else(|| "tool".into()),
                        "response": result.output,
                    }
                }),
                MessagePart::Json { value } => json!({ "text": value.to_string() }),
            })
        })
        .collect()
}
