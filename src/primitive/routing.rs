use std::collections::BTreeMap;

use serde_json::Value;

use crate::api::RequestBody;
use crate::protocol::AuthScheme;

use super::{PrimitiveProviderKind, PrimitiveRequest, ProviderPrimitiveWireFormat};

pub(super) fn default_headers_for(provider: PrimitiveProviderKind) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    if matches!(provider, PrimitiveProviderKind::Anthropic) {
        headers.insert("anthropic-version".into(), "2023-06-01".into());
    }
    headers
}

pub(super) fn default_auth_for(provider: PrimitiveProviderKind) -> AuthScheme {
    match provider {
        PrimitiveProviderKind::OpenAi
        | PrimitiveProviderKind::AzureOpenAi
        | PrimitiveProviderKind::OpenAiCompatible
        | PrimitiveProviderKind::Bedrock
        | PrimitiveProviderKind::Custom => AuthScheme::Bearer,
        PrimitiveProviderKind::Anthropic => AuthScheme::Header {
            name: "x-api-key".into(),
        },
        PrimitiveProviderKind::Gemini | PrimitiveProviderKind::VertexAi => AuthScheme::Header {
            name: "x-goog-api-key".into(),
        },
    }
}

pub(super) fn default_path(request: &PrimitiveRequest) -> Option<String> {
    match request.wire_format {
        ProviderPrimitiveWireFormat::OpenAiResponses => Some("/responses".into()),
        ProviderPrimitiveWireFormat::OpenAiChatCompletions
        | ProviderPrimitiveWireFormat::OpenAiCompatibleChatCompletions => {
            Some("/chat/completions".into())
        }
        ProviderPrimitiveWireFormat::OpenAiImages => Some("/images/generations".into()),
        ProviderPrimitiveWireFormat::OpenAiImageEdits => Some("/images/edits".into()),
        ProviderPrimitiveWireFormat::OpenAiImageVariations => Some("/images/variations".into()),
        ProviderPrimitiveWireFormat::OpenAiRealtime => Some("/realtime/sessions".into()),
        ProviderPrimitiveWireFormat::OpenAiAudioTranscriptions => {
            Some("/audio/transcriptions".into())
        }
        ProviderPrimitiveWireFormat::OpenAiAudioTranslations => Some("/audio/translations".into()),
        ProviderPrimitiveWireFormat::OpenAiAudioSpeech => Some("/audio/speech".into()),
        ProviderPrimitiveWireFormat::OpenAiEmbeddings => Some("/embeddings".into()),
        ProviderPrimitiveWireFormat::OpenAiFiles => Some("/files".into()),
        ProviderPrimitiveWireFormat::OpenAiUploads => Some("/uploads".into()),
        ProviderPrimitiveWireFormat::OpenAiModels => Some("/models".into()),
        ProviderPrimitiveWireFormat::OpenAiBatches => Some("/batches".into()),
        ProviderPrimitiveWireFormat::AnthropicMessages => Some("/messages".into()),
        ProviderPrimitiveWireFormat::AnthropicCountTokens => Some("/messages/count_tokens".into()),
        ProviderPrimitiveWireFormat::AnthropicMessageBatches => Some("/messages/batches".into()),
        ProviderPrimitiveWireFormat::AnthropicFiles => Some("/files".into()),
        ProviderPrimitiveWireFormat::AnthropicModels => Some("/models".into()),
        ProviderPrimitiveWireFormat::GeminiGenerateContent => {
            model_path(request, "generateContent")
        }
        ProviderPrimitiveWireFormat::GeminiStreamGenerateContent => {
            model_path(request, "streamGenerateContent")
        }
        ProviderPrimitiveWireFormat::GeminiCountTokens => model_path(request, "countTokens"),
        ProviderPrimitiveWireFormat::GeminiEmbedContent => model_path(request, "embedContent"),
        ProviderPrimitiveWireFormat::GeminiLive => None,
        ProviderPrimitiveWireFormat::GeminiFiles => Some("/files".into()),
        ProviderPrimitiveWireFormat::GeminiCaches => Some("/cachedContents".into()),
        ProviderPrimitiveWireFormat::GeminiModels => Some("/models".into()),
        ProviderPrimitiveWireFormat::GeminiOperations => Some("/operations".into()),
        ProviderPrimitiveWireFormat::GeminiBatches => Some("/batches".into()),
        ProviderPrimitiveWireFormat::BedrockConverse
        | ProviderPrimitiveWireFormat::BedrockInvokeModel
        | ProviderPrimitiveWireFormat::CustomHttp => None,
    }
}

fn model_path(request: &PrimitiveRequest, action: &str) -> Option<String> {
    let model = request.model.as_ref()?;
    Some(format!("/models/{model}:{action}"))
}

pub(super) fn estimate_body_tokens(body: &RequestBody) -> u32 {
    let chars = match body {
        RequestBody::Json { value } => value.to_string().len(),
        RequestBody::Multipart { fields } => fields
            .iter()
            .map(|field| match &field.value {
                crate::api::MultipartValue::Text { value } => value.len(),
                crate::api::MultipartValue::File { data_base64, .. } => data_base64.len() / 8,
            })
            .sum(),
        RequestBody::Text { text } => text.len(),
        RequestBody::Binary { data_base64, .. } => data_base64.len() / 8,
    };
    ((chars / 4).max(1)) as u32
}

pub(super) fn known_output_token_limit(value: &Value) -> Option<u32> {
    value
        .get("max_output_tokens")
        .or_else(|| value.get("max_tokens"))
        .or_else(|| value.get("maxOutputTokens"))
        .or_else(|| value.pointer("/generationConfig/maxOutputTokens"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}
