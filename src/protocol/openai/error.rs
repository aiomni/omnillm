use serde_json::Value;

use crate::error::ProviderError;
use crate::types::VendorExtensions;

use super::super::ProviderProtocol;

pub(in crate::protocol) fn parse_openai_error(
    protocol: ProviderProtocol,
    status: Option<u16>,
    body: &Value,
) -> ProviderError {
    ProviderError {
        protocol,
        status,
        code: body
            .get("error")
            .and_then(|value| value.get("code"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                body.get("error")
                    .and_then(|value| value.get("type"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }),
        message: body
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("provider error")
            .to_string(),
        retry_after: None,
        raw_body: Some(body.to_string()),
        vendor_extensions: VendorExtensions::new(),
    }
}
