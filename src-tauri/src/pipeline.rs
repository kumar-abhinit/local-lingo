use crate::asr::{postprocess, transcribe, validate_samples};
use crate::audio::{AudioCapture, SpeechBoundary, VadProcessor};
use crate::config::{AppConfig, HotkeyMode};
use crate::hotkey::{HotkeyEvent, HotkeyListener};
use crate::injection::{inject_with_fallback, platform_injector};
use crate::tray::TrayState;
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub struct Pipeline {
    state: Arc<Mutex<TrayState>>,
    config: Arc<Mutex<AppConfig>>,
    capture: Arc<Mutex<Option<AudioCapture>>>,
    vad: Arc<Mutex<Option<VadProcessor>>>,
    toggle_active: Arc<Mutex<bool>>,
    app: AppHandle,
}

impl Pipeline {
    pub fn new(app: AppHandle, config: AppConfig) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(TrayState::Idle)),
            config: Arc::new(Mutex::new(config)),
            capture: Arc::new(Mutex::new(None)),
            vad: Arc::new(Mutex::new(None)),
            toggle_active: Arc::new(Mutex::new(false)),
            app,
        })
    }

    pub fn current_state(&self) -> TrayState {
        *self.state.lock()
    }

    pub fn set_state(&self, state: TrayState) {
        *self.state.lock() = state;
        let _ = self.app.emit("pipeline-state", state);
        if let Some(tray) = self.app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(state.tooltip()));
        }
    }

    pub fn spawn_hotkey_listener(self: &Arc<Self>) -> Result<()> {
        let config = self.config.lock().clone();
        let (listener, rx) = match HotkeyListener::spawn(&config.hotkey, config.hotkey_mode) {
            Ok(pair) => pair,
            Err(e) => {
                log::error!("global hotkey unavailable: {e:#}");
                let _ = self.app.emit("hotkey-error", e.to_string());
                self.set_state(TrayState::Error);
                return Ok(());
            }
        };
        let pipeline = Arc::clone(self);

        thread::spawn(move || {
            for event in rx {
                if let Err(e) = pipeline.handle_hotkey(event) {
                    log::error!("hotkey handler error: {e:#}");
                    pipeline.set_state(TrayState::Error);
                    thread::sleep(Duration::from_secs(2));
                    pipeline.set_state(TrayState::Idle);
                }
            }
            listener.stop();
        });

        Ok(())
    }

    fn handle_hotkey(self: &Arc<Self>, event: HotkeyEvent) -> Result<()> {
        let mode = self.config.lock().hotkey_mode;
        match (mode, event) {
            (HotkeyMode::PushToTalk, HotkeyEvent::Pressed) => self.start_listening(),
            (HotkeyMode::PushToTalk, HotkeyEvent::Released) => self.stop_and_transcribe(),
            (HotkeyMode::Toggle, HotkeyEvent::Pressed) => {
                let mut active = self.toggle_active.lock();
                if *active {
                    *active = false;
                    self.stop_and_transcribe()?;
                } else {
                    *active = true;
                    self.start_listening()?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn start_listening(self: &Arc<Self>) -> Result<()> {
        if *self.state.lock() != TrayState::Idle {
            return Ok(());
        }

        let config = self.config.lock().clone();
        let mut capture = AudioCapture::new(config.mic_device.as_deref())?;
        capture.start()?;

        let vad = VadProcessor::new(config.trailing_silence_ms)?;

        *self.capture.lock() = Some(capture);
        *self.vad.lock() = Some(vad);
        self.set_state(TrayState::Listening);

        if config.hotkey_mode == HotkeyMode::Toggle {
            let pipeline = Arc::clone(self);
            thread::spawn(move || {
                if let Err(e) = pipeline.vad_poll_loop() {
                    log::error!("VAD poll error: {e:#}");
                }
            });
        }

        Ok(())
    }

    fn vad_poll_loop(self: &Arc<Self>) -> Result<()> {
        while *self.state.lock() == TrayState::Listening {
            thread::sleep(Duration::from_millis(50));

            let chunk = {
                let cap_guard = self.capture.lock();
                cap_guard
                    .as_ref()
                    .map(|c| c.drain_new_samples())
                    .unwrap_or_default()
            };

            if chunk.is_empty() {
                continue;
            }

            let ended = {
                let mut vad_guard = self.vad.lock();
                if let Some(vad) = vad_guard.as_mut() {
                    let events = vad.push_samples(&chunk)?;
                    events.contains(&SpeechBoundary::SpeechEnded)
                } else {
                    false
                }
            };

            if ended {
                *self.toggle_active.lock() = false;
                self.stop_and_transcribe()?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn stop_and_transcribe(self: &Arc<Self>) -> Result<()> {
        if *self.state.lock() != TrayState::Listening {
            return Ok(());
        }

        self.set_state(TrayState::Transcribing);

        let samples = {
            let mut cap = self.capture.lock();
            cap.take().map(|mut c| c.stop()).unwrap_or_default()
        };

        let mut vad_samples = {
            let mut vad_guard = self.vad.lock();
            vad_guard
                .as_mut()
                .map(|v| v.take_captured())
                .unwrap_or_default()
        };

        *self.vad.lock() = None;

        if vad_samples.is_empty() {
            vad_samples = samples;
        }

        if vad_samples.is_empty() {
            self.set_state(TrayState::Idle);
            return Ok(());
        }

        validate_samples(&vad_samples)?;
        let start = Instant::now();

        let raw = transcribe(&vad_samples)?;
        let text = postprocess(&raw);
        log::info!(
            "transcription ({:.0}ms): {text}",
            start.elapsed().as_millis()
        );

        if !text.is_empty() {
            let injector = platform_injector();
            inject_with_fallback(injector.as_ref(), &text)?;
            let _ = self.app.emit("transcription", &text);
        }

        self.set_state(TrayState::Idle);
        Ok(())
    }

    pub fn debug_record_and_transcribe(&self, seconds: f32) -> Result<String> {
        self.set_state(TrayState::Listening);
        let config = self.config.lock().clone();
        let samples = AudioCapture::record_for_seconds(config.mic_device.as_deref(), seconds)?;
        self.set_state(TrayState::Transcribing);
        validate_samples(&samples)?;
        let raw = transcribe(&samples)?;
        let text = postprocess(&raw);
        self.set_state(TrayState::Idle);
        Ok(text)
    }
}
