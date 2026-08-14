use crate::asr::{cloud, engine};
use crate::config::AppConfig;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AsrBackend {
    Local,
    Cloud,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrStatus {
    pub backend: AsrBackend,
    pub model_path: Option<String>,
    pub cloud_configured: bool,
    pub local_model_cached: bool,
}

pub fn resolve_local_model_path(cfg: &AppConfig) -> PathBuf {
    cfg.model_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::asr::model_path_for_tier(cfg.model_tier))
}

pub fn local_model_available(cfg: &AppConfig) -> bool {
    resolve_local_model_path(cfg).exists()
}

pub fn groq_key(cfg: &AppConfig) -> Option<String> {
    cfg.groq_api_key
        .as_ref()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .or_else(|| {
            std::env::var("GROQ_API_KEY")
                .ok()
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
        })
}

pub fn cloud_configured(cfg: &AppConfig) -> bool {
    cloud_available(cfg)
}

fn cloud_available(cfg: &AppConfig) -> bool {
    #[cfg(feature = "network-isolation")]
    {
        let _ = cfg;
        return false;
    }
    #[cfg(not(feature = "network-isolation"))]
    {
        groq_key(cfg).is_some()
    }
}

pub fn active_backend(cfg: &AppConfig) -> AsrBackend {
    if local_model_available(cfg) {
        AsrBackend::Local
    } else if cloud_available(cfg) {
        AsrBackend::Cloud
    } else {
        AsrBackend::Unavailable
    }
}

pub fn asr_status(cfg: &AppConfig) -> AsrStatus {
    let path = resolve_local_model_path(cfg);
    AsrStatus {
        backend: active_backend(cfg),
        model_path: if path.exists() {
            Some(path.to_string_lossy().into_owned())
        } else {
            None
        },
        cloud_configured: cloud_configured(cfg),
        local_model_cached: path.exists(),
    }
}

pub fn ensure_transcription_ready(cfg: &AppConfig) -> Result<()> {
    match active_backend(cfg) {
        AsrBackend::Local => init_local_engine(cfg),
        AsrBackend::Cloud => Ok(()),
        AsrBackend::Unavailable => Err(anyhow!(
            "No transcription backend available — download a local model or add a Groq API key in Settings"
        )),
    }
}

fn init_local_engine(cfg: &AppConfig) -> Result<()> {
    let path = resolve_local_model_path(cfg);
    if !path.exists() {
        return Err(anyhow!(
            "Model not found at {} — download via Settings or onboarding",
            path.display()
        ));
    }
    engine::init_engine(path.as_path())
}

pub fn transcribe(samples: &[f32], cfg: &AppConfig) -> Result<String> {
    match active_backend(cfg) {
        AsrBackend::Local => {
            init_local_engine(cfg)?;
            engine::transcribe(samples)
        }
        AsrBackend::Cloud => {
            let key = groq_key(cfg).ok_or_else(|| {
                anyhow!("Groq API key not configured — add one in Settings")
            })?;
            cloud::groq_transcribe_blocking(samples, &key)
        }
        AsrBackend::Unavailable => Err(anyhow!(
            "No transcription backend available — download a local model or add a Groq API key in Settings"
        )),
    }
}
