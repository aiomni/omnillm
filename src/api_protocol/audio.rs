use serde_json::{Map, Value};

use crate::api::{
    AudioInput, AudioSegment, AudioSpeechRequest, AudioTranscriptionRequest,
    AudioTranscriptionResponse, HttpMethod, MultipartField, MultipartValue, RequestBody,
    TranscribedWord, TransportRequest, WireFormat,
};

use super::common::*;
use super::generation::wire_path;
use super::ApiProtocolError;

pub(super) fn emit_openai_audio_transcription_transport(
    request: &AudioTranscriptionRequest,
) -> Result<TransportRequest, ApiProtocolError> {
    let mut fields = vec![MultipartField {
        name: "model".into(),
        value: MultipartValue::Text {
            value: request.model.clone(),
        },
    }];

    match &request.audio {
        AudioInput::File {
            filename,
            data_base64,
            media_type,
        } => fields.push(MultipartField {
            name: "file".into(),
            value: MultipartValue::File {
                filename: filename.clone(),
                data_base64: data_base64.clone(),
                media_type: media_type.clone(),
            },
        }),
        AudioInput::Url { .. } => {
            return Err(ApiProtocolError::UnsupportedFeature {
                wire_format: WireFormat::OpenAiAudioTranscriptions,
                message: "audio URL inputs are not supported by multipart transcription requests"
                    .into(),
            })
        }
    }

    if let Some(prompt) = &request.prompt {
        fields.push(MultipartField {
            name: "prompt".into(),
            value: MultipartValue::Text {
                value: prompt.clone(),
            },
        });
    }
    if let Some(response_format) = &request.response_format {
        fields.push(MultipartField {
            name: "response_format".into(),
            value: MultipartValue::Text {
                value: response_format.clone(),
            },
        });
    }
    if let Some(language) = &request.language {
        fields.push(MultipartField {
            name: "language".into(),
            value: MultipartValue::Text {
                value: language.clone(),
            },
        });
    }
    if let Some(temperature) = request.temperature {
        fields.push(MultipartField {
            name: "temperature".into(),
            value: MultipartValue::Text {
                value: temperature.to_string(),
            },
        });
    }
    for granularity in &request.timestamp_granularities {
        fields.push(MultipartField {
            name: "timestamp_granularities[]".into(),
            value: MultipartValue::Text {
                value: granularity.clone(),
            },
        });
    }

    Ok(TransportRequest {
        method: HttpMethod::Post,
        path: wire_path(WireFormat::OpenAiAudioTranscriptions, &request.model),
        headers: Default::default(),
        accept: Some("application/json".into()),
        body: RequestBody::Multipart { fields },
    })
}

pub(super) fn emit_openai_audio_speech_request(request: &AudioSpeechRequest) -> Value {
    let mut map = Map::new();
    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert("input".into(), Value::String(request.input.clone()));
    map.insert("voice".into(), Value::String(request.voice.clone()));
    if let Some(response_format) = &request.response_format {
        map.insert(
            "response_format".into(),
            Value::String(response_format.clone()),
        );
    }
    if let Some(speed) = request.speed {
        map.insert("speed".into(), Value::from(speed));
    }
    extend_with_vendor_extensions(&mut map, &request.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_audio_speech_request(
    body: &Value,
) -> Result<AudioSpeechRequest, ApiProtocolError> {
    Ok(AudioSpeechRequest {
        model: required_str(body, "model")?.to_string(),
        input: required_str(body, "input")?.to_string(),
        voice: required_str(body, "voice")?.to_string(),
        response_format: body
            .get("response_format")
            .and_then(Value::as_str)
            .map(str::to_owned),
        speed: body
            .get("speed")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        vendor_extensions: collect_vendor_extensions(
            body,
            &["model", "input", "voice", "response_format", "speed"],
        ),
    })
}

pub(super) fn emit_openai_audio_transcription_response(
    response: &AudioTranscriptionResponse,
) -> Value {
    let mut map = Map::new();
    map.insert("text".into(), Value::String(response.text.clone()));
    if let Some(language) = &response.language {
        map.insert("language".into(), Value::String(language.clone()));
    }
    if let Some(duration) = response.duration_seconds {
        map.insert("duration".into(), Value::from(duration));
    }
    if !response.segments.is_empty() {
        map.insert(
            "segments".into(),
            Value::Array(
                response
                    .segments
                    .iter()
                    .map(|segment| {
                        let mut segment_map = Map::new();
                        if let Some(id) = segment.id {
                            segment_map.insert("id".into(), Value::from(id));
                        }
                        if let Some(start) = segment.start {
                            segment_map.insert("start".into(), Value::from(start));
                        }
                        if let Some(end) = segment.end {
                            segment_map.insert("end".into(), Value::from(end));
                        }
                        segment_map.insert("text".into(), Value::String(segment.text.clone()));
                        Value::Object(segment_map)
                    })
                    .collect(),
            ),
        );
    }
    if !response.words.is_empty() {
        map.insert(
            "words".into(),
            Value::Array(
                response
                    .words
                    .iter()
                    .map(|word| {
                        let mut word_map = Map::new();
                        word_map.insert("word".into(), Value::String(word.word.clone()));
                        if let Some(start) = word.start {
                            word_map.insert("start".into(), Value::from(start));
                        }
                        if let Some(end) = word.end {
                            word_map.insert("end".into(), Value::from(end));
                        }
                        Value::Object(word_map)
                    })
                    .collect(),
            ),
        );
    }
    extend_with_vendor_extensions(&mut map, &response.vendor_extensions);
    Value::Object(map)
}

pub(super) fn parse_openai_audio_transcription_response(
    body: &Value,
) -> Result<AudioTranscriptionResponse, ApiProtocolError> {
    let segments = body
        .get("segments")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .map(|segment| AudioSegment {
                    id: segment
                        .get("id")
                        .and_then(Value::as_u64)
                        .map(|value| value as u32),
                    start: segment
                        .get("start")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                    end: segment
                        .get("end")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                    text: segment
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let words = body
        .get("words")
        .and_then(Value::as_array)
        .map(|words| {
            words
                .iter()
                .map(|word| TranscribedWord {
                    word: word
                        .get("word")
                        .or_else(|| word.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    start: word
                        .get("start")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                    end: word
                        .get("end")
                        .and_then(Value::as_f64)
                        .map(|value| value as f32),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(AudioTranscriptionResponse {
        text: required_str(body, "text")?.to_string(),
        language: body
            .get("language")
            .and_then(Value::as_str)
            .map(str::to_owned),
        duration_seconds: body
            .get("duration")
            .or_else(|| body.get("duration_seconds"))
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        segments,
        words,
        vendor_extensions: collect_vendor_extensions(
            body,
            &[
                "text",
                "language",
                "duration",
                "duration_seconds",
                "segments",
                "words",
            ],
        ),
    })
}
