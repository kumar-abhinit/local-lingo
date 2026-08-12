use crate::config::TARGET_SAMPLE_RATE;
use anyhow::{Context, Result};
use silero_vad_rust::load_silero_vad;
use silero_vad_rust::silero_vad::utils_vad::{VadEvent, VadIterator, VadIteratorParams};

const FRAME_SIZE: usize = 512;
const PRE_ROLL_MS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechBoundary {
    SpeechStarted,
    SpeechEnded,
}

pub struct VadProcessor {
    iterator: VadIterator,
    frame_buffer: Vec<f32>,
    pre_roll: Vec<f32>,
    pre_roll_capacity: usize,
    speech_active: bool,
    captured: Vec<f32>,
}

impl VadProcessor {
    pub fn new(trailing_silence_ms: u64) -> Result<Self> {
        let model = load_silero_vad().context("failed to load Silero VAD model")?;
        let params = VadIteratorParams {
            threshold: 0.5,
            min_silence_duration_ms: trailing_silence_ms as u32,
            speech_pad_ms: 30,
            ..Default::default()
        };
        let iterator = VadIterator::new(model, params).context("failed to create VAD iterator")?;

        let pre_roll_capacity = (TARGET_SAMPLE_RATE as u64 * PRE_ROLL_MS / 1000) as usize;

        Ok(Self {
            iterator,
            frame_buffer: Vec::with_capacity(FRAME_SIZE),
            pre_roll: Vec::with_capacity(pre_roll_capacity),
            pre_roll_capacity,
            speech_active: false,
            captured: Vec::new(),
        })
    }

    pub fn reset(&mut self) {
        self.frame_buffer.clear();
        self.pre_roll.clear();
        self.speech_active = false;
        self.captured.clear();
    }

    pub fn push_samples(&mut self, samples: &[f32]) -> Result<Vec<SpeechBoundary>> {
        let mut events = Vec::new();

        for &sample in samples {
            self.push_pre_roll(sample);
            self.frame_buffer.push(sample);

            if self.frame_buffer.len() < FRAME_SIZE {
                continue;
            }

            let frame: Vec<f32> = self.frame_buffer.drain(..).collect();
            let event = self
                .iterator
                .process_chunk(&frame, true, 1)
                .context("VAD process_chunk failed")?;

            match event {
                Some(VadEvent::Start(_)) => {
                    if !self.speech_active {
                        self.speech_active = true;
                        self.captured.extend(self.pre_roll.drain(..));
                        events.push(SpeechBoundary::SpeechStarted);
                    }
                    self.captured.extend_from_slice(&frame);
                }
                Some(VadEvent::End(_)) => {
                    self.captured.extend_from_slice(&frame);
                    if self.speech_active {
                        self.speech_active = false;
                        events.push(SpeechBoundary::SpeechEnded);
                    }
                }
                None => {
                    if self.speech_active {
                        self.captured.extend_from_slice(&frame);
                    }
                }
            }
        }

        Ok(events)
    }

    pub fn take_captured(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.captured)
    }

    pub fn is_speech_active(&self) -> bool {
        self.speech_active
    }

    fn push_pre_roll(&mut self, sample: f32) {
        self.pre_roll.push(sample);
        if self.pre_roll.len() > self.pre_roll_capacity {
            self.pre_roll.remove(0);
        }
    }
}
