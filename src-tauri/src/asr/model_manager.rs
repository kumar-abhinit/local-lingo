use crate::config::{models_dir, ModelTier};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub filename: String,
    pub tier: ModelTier,
    pub size_mb: u32,
    pub url: String,
    pub sha256: Option<String>,
    pub cached: bool,
}

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

fn catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "small.en".into(),
            filename: "ggml-small.en-q5_0.bin".into(),
            tier: ModelTier::Low,
            size_mb: 190,
            url: format!("{HF_BASE}/ggml-small.en-q5_0.bin"),
            sha256: None,
            cached: false,
        },
        ModelInfo {
            id: "medium.en".into(),
            filename: "ggml-medium.en-q5_0.bin".into(),
            tier: ModelTier::Medium,
            size_mb: 540,
            url: format!("{HF_BASE}/ggml-medium.en-q5_0.bin"),
            sha256: None,
            cached: false,
        },
        ModelInfo {
            id: "large-v3-turbo".into(),
            filename: "ggml-large-v3-turbo-q5_0.bin".into(),
            tier: ModelTier::High,
            size_mb: 550,
            url: format!("{HF_BASE}/ggml-large-v3-turbo-q5_0.bin"),
            sha256: None,
            cached: false,
        },
    ]
}

pub fn list_available_models() -> Result<Vec<ModelInfo>> {
    let dir = models_dir();
    fs::create_dir_all(&dir)?;
    Ok(catalog()
        .into_iter()
        .map(|mut m| {
            m.cached = dir.join(&m.filename).exists();
            m
        })
        .collect())
}

pub fn model_path_for_tier(tier: ModelTier) -> PathBuf {
    let filename = match tier {
        ModelTier::Low => "ggml-small.en-q5_0.bin",
        ModelTier::Medium => "ggml-medium.en-q5_0.bin",
        ModelTier::High => "ggml-large-v3-turbo-q5_0.bin",
    };
    models_dir().join(filename)
}

pub fn model_path(model_id: &str) -> Result<PathBuf> {
    let model = catalog()
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| anyhow!("unknown model id: {model_id}"))?;
    Ok(models_dir().join(model.filename))
}

#[cfg(feature = "model-download")]
pub async fn download_model(
    model_id: &str,
    progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
) -> Result<PathBuf> {
    let model = catalog()
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| anyhow!("unknown model id: {model_id}"))?;

    let dir = models_dir();
    fs::create_dir_all(&dir)?;
    let dest = dir.join(&model.filename);
    let tmp = dir.join(format!("{}.part", model.filename));

    if dest.exists() {
        return Ok(dest);
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let response = client.get(&model.url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!("download failed: HTTP {}", response.status()));
    }

    let total = response.content_length();
    let bytes = response.bytes().await.context("download read error")?;
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&bytes)?;
    progress(bytes.len() as u64, total);
    file.sync_all()?;
    drop(file);

    if let Some(expected) = &model.sha256 {
        verify_checksum(&tmp, expected)?;
    }

    fs::rename(&tmp, &dest)?;
    Ok(dest)
}

#[cfg(not(feature = "model-download"))]
pub async fn download_model(
    _model_id: &str,
    _progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
) -> Result<PathBuf> {
    Err(anyhow!(
        "model download disabled (build without network-isolation, with model-download feature)"
    ))
}

pub fn ensure_model_cached(tier: ModelTier) -> Result<PathBuf> {
    let path = model_path_for_tier(tier);
    if path.exists() {
        return Ok(path);
    }
    Err(anyhow!(
        "model not cached at {} — run onboarding to download",
        path.display()
    ))
}

fn verify_checksum(path: &Path, expected_hex: &str) -> Result<()> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(&bytes);
    let hex = hex::encode(digest);
    if hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        fs::remove_file(path).ok();
        Err(anyhow!("checksum mismatch for {}", path.display()))
    }
}

#[cfg(feature = "model-download")]
mod network_guard {
    /// Compile-time marker: reqwest must only be used in this module.
    pub const NETWORK_ALLOWED: bool = true;
}

#[cfg(feature = "network-isolation")]
pub fn assert_network_isolation() {
    #[cfg(feature = "model-download")]
    {
        log::warn!(
            "network-isolation feature enabled alongside model-download — downloads still possible"
        );
    }
}
