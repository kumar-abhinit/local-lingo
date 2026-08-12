use crate::config::{ModelTier, TARGET_SAMPLE_RATE};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::OnceLock;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
};

static ENGINE: OnceLock<Mutex<Option<LoadedEngine>>> = OnceLock::new();

struct LoadedEngine {
    model_path: std::path::PathBuf,
    engine: AsrEngine,
}

fn engine_slot() -> &'static Mutex<Option<LoadedEngine>> {
    ENGINE.get_or_init(|| Mutex::new(None))
}

pub struct AsrEngine {
    context: WhisperContext,
}

impl AsrEngine {
    pub fn load(model_path: &Path) -> Result<Self> {
        let path_str = model_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid model path"))?;
        let context = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
            .context("failed to load whisper model")?;
        Ok(Self { context })
    }

    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }

        let mut state = self
            .context
            .create_state()
            .context("failed to create whisper state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(num_cpus());
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_single_segment(true);

        state
            .full(params, samples)
            .context("whisper inference failed")?;

        let n = state.full_n_segments();
        let mut text = String::new();
        for i in 0..n {
            if let Some(segment) = state.get_segment(i) {
                let segment_text = segment
                    .to_str_lossy()
                    .map_err(|e| anyhow!("failed to read segment: {e}"))?;
                text.push_str(&segment_text);
            }
        }
        Ok(text.trim().to_string())
    }
}

fn num_cpus() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1) as i32)
        .unwrap_or(1)
}

pub fn init_engine(model_path: &Path) -> Result<()> {
    let mut slot = engine_slot().lock();
    if let Some(loaded) = slot.as_ref() {
        if loaded.model_path == model_path {
            return Ok(());
        }
    }

    let engine = AsrEngine::load(model_path)?;
    *slot = Some(LoadedEngine {
        model_path: model_path.to_path_buf(),
        engine,
    });
    Ok(())
}

pub fn transcribe(samples: &[f32]) -> Result<String> {
    let slot = engine_slot().lock();
    let loaded = slot
        .as_ref()
        .ok_or_else(|| anyhow!("ASR engine not initialized"))?;
    loaded.engine.transcribe(samples)
}

/// Ensure samples are 16kHz mono f32 (capture module already provides this).
pub fn validate_samples(samples: &[f32]) -> Result<()> {
    if samples.len() < TARGET_SAMPLE_RATE as usize / 10 {
        return Err(anyhow!("audio too short for transcription"));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).context("failed to open wav")?;
    let spec = reader.spec();
    if spec.sample_rate != TARGET_SAMPLE_RATE {
        return Err(anyhow!(
            "expected {} Hz, got {} Hz",
            TARGET_SAMPLE_RATE,
            spec.sample_rate
        ));
    }

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.context("sample read error"))
            .collect::<Result<_>>()?,
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| Ok(s.context("sample read error")? as f32 / i16::MAX as f32))
            .collect::<Result<_>>()?,
    };

    if spec.channels == 2 {
        return Ok(samples
            .chunks(2)
            .map(|c| (c[0] + c.get(1).copied().unwrap_or(0.0)) / 2.0)
            .collect());
    }

    Ok(samples)
}
