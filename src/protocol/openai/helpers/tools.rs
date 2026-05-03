use serde_json::{json, Value};

use crate::types::{ToolDefinition, VendorExtensions};

use super::super::super::common::required_str;
use super::super::super::ProtocolError;

pub(in crate::protocol::openai) fn parse_function_tools(
    value: Option<&Value>,
) -> Result<Vec<ToolDefinition>, ProtocolError> {
    let Some(Value::Array(tools)) = value else {
        return Ok(Vec::new());
    };

    tools
        .iter()
        .map(|tool| {
            let function = if tool.get("function").is_some() {
                tool.get("function").unwrap_or(tool)
            } else {
                tool
            };
            Ok(ToolDefinition {
                name: required_str(function, "name")?.to_string(),
                description: function
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                input_schema: function
                    .get("parameters")
                    .or_else(|| function.get("input_schema"))
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                strict: function
                    .get("strict")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                vendor_extensions: VendorExtensions::new(),
            })
        })
        .collect()
}

pub(in crate::protocol::openai) fn emit_function_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                    "strict": tool.strict,
                }
            })
        })
        .collect()
}

pub(in crate::protocol::openai) fn emit_openai_responses_function_tools(
    tools: &[ToolDefinition],
) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
                "strict": tool.strict,
            })
        })
        .collect()
}
