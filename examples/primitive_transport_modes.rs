//! Demonstrates P3 primitive transport request construction without network I/O.
//!
//! This example intentionally only builds provider-native requests. Runtime
//! execution still requires a configured `Gateway`, provider endpoint, and API key.

use omnillm::{
    PrimitiveEndpointKind, PrimitiveProviderKind, PrimitiveRequest, PrimitiveStreamMode,
    ProviderPrimitiveWireFormat,
};
use serde_json::json;

fn main() {
    let audio_binary_stream = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::AudioSpeech,
        ProviderPrimitiveWireFormat::OpenAiAudioSpeech,
        "gpt-4o-mini-tts",
        json!({"model":"gpt-4o-mini-tts","voice":"alloy","input":"hello"}),
    )
    .with_stream(PrimitiveStreamMode::BinaryChunks);

    let openai_realtime = PrimitiveRequest::json(
        PrimitiveProviderKind::OpenAi,
        PrimitiveEndpointKind::Realtime,
        ProviderPrimitiveWireFormat::OpenAiRealtime,
        "gpt-4o-realtime-preview",
        json!({"type":"session.update","session":{"modalities":["text"]}}),
    )
    .with_stream(PrimitiveStreamMode::WebSocket);

    let gemini_live = PrimitiveRequest::json(
        PrimitiveProviderKind::Gemini,
        PrimitiveEndpointKind::Live,
        ProviderPrimitiveWireFormat::GeminiLive,
        "gemini-2.5-flash",
        json!({"setup":{"model":"models/gemini-2.5-flash"}}),
    )
    .with_path("/live")
    .with_stream(PrimitiveStreamMode::WebSocket);

    println!(
        "transport modes: {:?}, {:?}, {:?}",
        audio_binary_stream.stream, openai_realtime.stream, gemini_live.stream
    );
}
