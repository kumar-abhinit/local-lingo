use crate::config::TARGET_SAMPLE_RATE;
use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream};
use crossbeam_channel::{Receiver, Sender};
use rubato::{FftFixedIn, Resampler};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const RING_CAPACITY_SAMPLES: usize = 480_000; // ~30s at 16kHz

#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

enum CaptureCommand {
    Start { device: Option<String> },
    Stop { reply: Sender<Vec<f32>> },
    Drain { reply: Sender<Vec<f32>> },
    Shutdown,
}

/// Send-safe handle; cpal `Stream` lives on a dedicated worker thread.
pub struct AudioCapture {
    tx: Sender<CaptureCommand>,
    selected_device: Option<String>,
    device_name: String,
}

impl AudioCapture {
    pub fn new(device_name: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();
        let device = resolve_device(&host, device_name)?;
        let device_label = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let initial_device = device_name.map(String::from);
        let (tx, rx) = crossbeam_channel::unbounded();
        thread::spawn(move || capture_worker(rx, initial_device));
        Ok(Self {
            tx,
            selected_device: device_name.map(String::from),
            device_name: device_label,
        })
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn start(&mut self) -> Result<()> {
        self.tx
            .send(CaptureCommand::Start {
                device: self.selected_device.clone(),
            })
            .map_err(|e| anyhow!("audio worker unavailable: {e}"))?;
        Ok(())
    }

    pub fn start_with_device(&mut self, device: Option<String>) -> Result<()> {
        self.tx
            .send(CaptureCommand::Start { device })
            .map_err(|e| anyhow!("audio worker unavailable: {e}"))?;
        Ok(())
    }

    pub fn stop(&mut self) -> Vec<f32> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        if self.tx.send(CaptureCommand::Stop { reply: reply_tx }).is_err() {
            return Vec::new();
        }
        reply_rx.recv().unwrap_or_default()
    }

    pub fn drain_new_samples(&self) -> Vec<f32> {
        let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
        if self
            .tx
            .send(CaptureCommand::Drain { reply: reply_tx })
            .is_err()
        {
            return Vec::new();
        }
        reply_rx.recv().unwrap_or_default()
    }

    pub fn is_recording(&self) -> bool {
        !self.drain_new_samples().is_empty()
    }

    pub fn record_for_seconds(device_name: Option<&str>, seconds: f32) -> Result<Vec<f32>> {
        let mut capture = Self::new(device_name)?;
        if let Some(name) = device_name {
            capture.start_with_device(Some(name.to_string()))?;
        } else {
            capture.start()?;
        }
        thread::sleep(Duration::from_secs_f32(seconds));
        Ok(capture.stop())
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.tx.send(CaptureCommand::Shutdown);
    }
}

struct CaptureWorker {
    selected_device: Option<String>,
    recording: Arc<AtomicBool>,
    samples: Arc<parking_lot::Mutex<Vec<f32>>>,
    drain_offset: Arc<parking_lot::Mutex<usize>>,
    stream: Option<Stream>,
}

impl CaptureWorker {
    fn new() -> Self {
        Self {
            selected_device: None,
            recording: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(parking_lot::Mutex::new(Vec::with_capacity(RING_CAPACITY_SAMPLES))),
            drain_offset: Arc::new(parking_lot::Mutex::new(0)),
            stream: None,
        }
    }

    fn handle(&mut self, cmd: CaptureCommand) {
        match cmd {
            CaptureCommand::Start { device } => {
                if let Some(device) = device {
                    self.selected_device = Some(device);
                }
                if let Err(e) = self.start_internal() {
                    log::error!("audio start failed: {e:#}");
                }
            }
            CaptureCommand::Stop { reply } => {
                let samples = self.stop_internal();
                let _ = reply.send(samples);
            }
            CaptureCommand::Drain { reply } => {
                let _ = reply.send(self.drain_internal());
            }
            CaptureCommand::Shutdown => {
                self.stop_internal();
            }
        }
    }

    fn start_internal(&mut self) -> Result<()> {
        if self.recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = resolve_device(&host, self.selected_device.as_deref())?;
        let config = device
            .default_input_config()
            .context("no default input config")?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let sample_format = config.sample_format();

        {
            let mut buf = self.samples.lock();
            buf.clear();
            buf.reserve(RING_CAPACITY_SAMPLES);
        }
        *self.drain_offset.lock() = 0;
        self.recording.store(true, Ordering::SeqCst);

        let recording = Arc::clone(&self.recording);
        let samples = Arc::clone(&self.samples);

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config.into(),
                sample_rate,
                channels,
                recording,
                samples,
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config.into(),
                sample_rate,
                channels,
                recording,
                samples,
            )?,
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config.into(),
                sample_rate,
                channels,
                recording,
                samples,
            )?,
            other => return Err(anyhow!("unsupported sample format: {other:?}")),
        };

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    fn stop_internal(&mut self) -> Vec<f32> {
        self.recording.store(false, Ordering::SeqCst);
        self.stream = None;
        *self.drain_offset.lock() = 0;
        std::mem::take(&mut *self.samples.lock())
    }

    fn drain_internal(&self) -> Vec<f32> {
        let buf = self.samples.lock();
        let mut offset = self.drain_offset.lock();
        if *offset >= buf.len() {
            return Vec::new();
        }
        let chunk = buf[*offset..].to_vec();
        *offset = buf.len();
        chunk
    }
}

fn capture_worker(rx: Receiver<CaptureCommand>, initial_device: Option<String>) {
    let mut worker = CaptureWorker::new();
    worker.selected_device = initial_device;
    for cmd in rx {
        if matches!(cmd, CaptureCommand::Shutdown) {
            worker.handle(cmd);
            break;
        }
        worker.handle(cmd);
    }
}

pub fn list_devices() -> Result<Vec<AudioDeviceInfo>> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok());

    host.input_devices()
        .context("failed to enumerate input devices")?
        .map(|device| {
            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            let is_default = default_name.as_deref() == Some(name.as_str());
            Ok(AudioDeviceInfo { name, is_default })
        })
        .collect()
}

fn resolve_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device> {
    if let Some(name) = name {
        for device in host.input_devices()? {
            if device.name().ok().as_deref() == Some(name) {
                return Ok(device);
            }
        }
        return Err(anyhow!("input device not found: {name}"));
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!("no default input device"))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    input_rate: u32,
    channels: usize,
    recording: Arc<AtomicBool>,
    samples: Arc<parking_lot::Mutex<Vec<f32>>>,
) -> Result<Stream>
where
    T: Sample + cpal::SizedSample + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let mut resampler = create_resampler(input_rate, channels)?;
    let chunk_size = resampler.input_frames_next();

    let err_fn = |err| log::error!("audio stream error: {err}");

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            if !recording.load(Ordering::SeqCst) {
                return;
            }

            let mono: Vec<f32> = if channels == 1 {
                data.iter().map(|s| f32::from_sample(*s)).collect()
            } else {
                data.chunks(channels)
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|s| f32::from_sample(*s))
                            .sum::<f32>()
                            / channels as f32
                    })
                    .collect()
            };

            if input_rate == TARGET_SAMPLE_RATE {
                append_samples(&samples, &mono);
                return;
            }

            for chunk in mono.chunks(chunk_size) {
                if chunk.len() < chunk_size {
                    break;
                }
                let input = vec![chunk.to_vec()];
                if let Ok(out) = resampler.process(&input, None) {
                    if let Some(channel) = out.first() {
                        append_samples(&samples, channel);
                    }
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn create_resampler(input_rate: u32, channels: usize) -> Result<FftFixedIn<f32>> {
    FftFixedIn::<f32>::new(
        input_rate as usize,
        TARGET_SAMPLE_RATE as usize,
        1024,
        channels.max(1),
        1,
    )
    .context("failed to create resampler")
}

fn append_samples(samples: &Arc<parking_lot::Mutex<Vec<f32>>>, chunk: &[f32]) {
    let mut buf = samples.lock();
    let remaining = RING_CAPACITY_SAMPLES.saturating_sub(buf.len());
    if remaining == 0 {
        return;
    }
    let take = chunk.len().min(remaining);
    buf.extend_from_slice(&chunk[..take]);
}

pub fn save_wav(path: &std::path::Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_sample = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(int_sample)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_devices_does_not_panic() {
        let _ = list_devices();
    }
}
