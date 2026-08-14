use crate::config::TARGET_SAMPLE_RATE;
use anyhow::{anyhow, Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;

const GROQ_TRANSCRIPTIONS_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const GROQ_MODEL: &str = "whisper-large-v3-turbo";

fn samples_to_wav_bytes(samples: &[f32]) -> Result<Vec<u8>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)?;
        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            writer.write_sample((clamped * i16::MAX as f32) as i16)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

#[derive(serde::Deserialize)]
struct GroqTranscriptionResponse {
    text: String,
}

#[cfg(feature = "model-download")]
pub async fn groq_transcribe(samples: &[f32], api_key: &str) -> Result<String> {
    if samples.is_empty() {
        return Ok(String::new());
    }

    let wav = samples_to_wav_bytes(samples)?;
    let file_part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .context("failed to build wav multipart part")?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", GROQ_MODEL)
        .text("language", "en")
        .text("response_format", "json");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let response = client
        .post(GROQ_TRANSCRIPTIONS_URL)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("Groq request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq API error ({status}): {body}"));
    }

    let parsed: GroqTranscriptionResponse = response
        .json()
        .await
        .context("failed to parse Groq response")?;

    Ok(parsed.text.trim().to_string())
}

#[cfg(not(feature = "model-download"))]
pub async fn groq_transcribe(_samples: &[f32], _api_key: &str) -> Result<String> {
    Err(anyhow!("cloud transcription disabled at build time"))
}

#[cfg(feature = "model-download")]
pub fn groq_transcribe_blocking(samples: &[f32], api_key: &str) -> Result<String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(groq_transcribe(samples, api_key)),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().context("failed to start tokio runtime")?;
            rt.block_on(groq_transcribe(samples, api_key))
        }
    }
}

#[cfg(not(feature = "model-download"))]
pub fn groq_transcribe_blocking(_samples: &[f32], _api_key: &str) -> Result<String> {
    Err(anyhow!("cloud transcription disabled at build time"))
}

#[cfg(all(test, feature = "model-download"))]
mod tests {
    use super::*;

    #[test]
    fn wav_bytes_non_empty_for_tone() {
        let samples: Vec<f32> = (0..1600).map(|i| (i as f32 * 0.01).sin()).collect();
        let wav = samples_to_wav_bytes(&samples).unwrap();
        assert!(wav.len() > 44);
    }
}
