mod asr;
mod audio;
mod config;
mod hotkey;
mod injection;
mod instance;
mod pipeline;
mod tray;

#[cfg(target_os = "linux")]
mod x11_shim;

use crate::asr::{
    download_model, ensure_transcription_ready, list_available_models, transcribe, AsrStatus,
};
use crate::audio::{list_devices, save_wav, AudioCapture};
use crate::config::{AppConfig, ModelTier};
use crate::injection::{platform_injector, inject_with_fallback, PermissionStatus};
use crate::pipeline::Pipeline;
use crate::tray::TrayState;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WebviewWindowBuilder,
};

use std::sync::OnceLock;

static PIPELINE: OnceLock<Mutex<Option<Arc<Pipeline>>>> = OnceLock::new();

fn pipeline_slot() -> &'static Mutex<Option<Arc<Pipeline>>> {
    PIPELINE.get_or_init(|| Mutex::new(None))
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
}

#[derive(Clone, Serialize)]
struct DownloadProgress {
    downloaded: u64,
    total: Option<u64>,
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.lock().clone()
}

#[tauri::command]
fn set_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    crate::config::save_config(&config).map_err(|e| e.to_string())?;
    *state.config.lock() = config;
    Ok(())
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<audio::AudioDeviceInfo>, String> {
    list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_models() -> Result<Vec<asr::ModelInfo>, String> {
    list_available_models().map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_model_cmd(app: AppHandle, model_id: String) -> Result<String, String> {
    let app_clone = app.clone();
    let path = download_model(&model_id, move |downloaded, total| {
        let _ = app_clone.emit(
            "download-progress",
            DownloadProgress { downloaded, total },
        );
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_permissions() -> PermissionStatus {
    platform_injector().check_permissions()
}

#[tauri::command]
fn test_injection(text: String) -> Result<(), String> {
    let injector = platform_injector();
    inject_with_fallback(injector.as_ref(), &text).map_err(|e| e.to_string())
}

#[tauri::command]
async fn debug_record_wav(
    state: State<'_, AppState>,
    duration_secs: f32,
) -> Result<String, String> {
    let cfg = state.config.lock().clone();
    let samples = AudioCapture::record_for_seconds(cfg.mic_device.as_deref(), duration_secs)
        .map_err(|e| e.to_string())?;

    let dir = crate::config::data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("debug-recording-{}.wav", unix_timestamp()));
    save_wav(&path, &samples).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_asr_status(state: State<'_, AppState>) -> AsrStatus {
    let cfg = state.config.lock().clone();
    crate::asr::asr_status(&cfg)
}

#[tauri::command]
async fn run_benchmark(state: State<'_, AppState>) -> Result<BenchmarkResult, String> {
    let cfg = state.config.lock().clone();
    if !crate::asr::local_model_available(&cfg) {
        return Err(
            "Benchmark requires a local model — download one in Settings or skip this step"
                .into(),
        );
    }
    ensure_transcription_ready(&cfg).map_err(|e| e.to_string())?;

    let samples = generate_test_tone(10.0);
    let start = std::time::Instant::now();
    let _ = transcribe(&samples, &cfg).map_err(|e| e.to_string())?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(BenchmarkResult {
        elapsed_ms,
        recommended_tier: recommend_tier(elapsed_ms),
    })
}

#[tauri::command]
async fn mic_test_transcribe(state: State<'_, AppState>, seconds: f32) -> Result<String, String> {
    use crate::asr::{postprocess, validate_samples};
    let cfg = state.config.lock().clone();
    ensure_transcription_ready(&cfg).map_err(|e| e.to_string())?;
    let samples = AudioCapture::record_for_seconds(cfg.mic_device.as_deref(), seconds)
        .map_err(|e| e.to_string())?;
    validate_samples(&samples).map_err(|e| e.to_string())?;
    let raw = transcribe(&samples, &cfg).map_err(|e| e.to_string())?;
    Ok(postprocess(&raw))
}

#[tauri::command]
fn complete_onboarding(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().clone();
    cfg.onboarding_complete = true;
    crate::config::save_config(&cfg).map_err(|e| e.to_string())?;
    *state.config.lock() = cfg.clone();

    if pipeline_slot().lock().is_none() {
        let pipeline = init_pipeline(&app, cfg).map_err(|e| e.to_string())?;
        *pipeline_slot().lock() = Some(pipeline);
    }
    Ok(())
}

#[derive(Serialize)]
struct BenchmarkResult {
    elapsed_ms: u64,
    recommended_tier: ModelTier,
}

fn recommend_tier(elapsed_ms: u64) -> ModelTier {
    if elapsed_ms < 1200 {
        ModelTier::High
    } else if elapsed_ms < 2500 {
        ModelTier::Medium
    } else {
        ModelTier::Low
    }
}

fn generate_test_tone(seconds: f32) -> Vec<f32> {
    let n = (crate::config::TARGET_SAMPLE_RATE as f32 * seconds) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / crate::config::TARGET_SAMPLE_RATE as f32;
            (440.0 * 2.0 * std::f32::consts::PI * t).sin() * 0.1
        })
        .collect()
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn setup_tray(app: &AppHandle) -> Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let mic_test = MenuItem::with_id(app, "mic_test", "Mic Test", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&settings, &mic_test, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip(TrayState::Idle.tooltip())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => open_settings(app),
            "mic_test" => {
                let _ = app.emit("mic-test-request", ());
                open_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_settings(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let _ = WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
        .title("LocalLingo Settings")
        .inner_size(640.0, 720.0)
        .build();
}

fn init_pipeline(app: &AppHandle, cfg: AppConfig) -> Result<Arc<Pipeline>> {
    ensure_transcription_ready(&cfg).map_err(|e| anyhow!(e))?;
    let pipeline = Arc::new(Pipeline::new(app.clone(), cfg)?);
    pipeline.spawn_hotkey_listener()?;
    Ok(pipeline)
}

fn should_show_settings_on_launch() -> bool {
    std::env::args().any(|arg| arg == "--show-settings")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    #[cfg(target_os = "linux")]
    crate::x11_shim::install_grab_error_handler();

    #[cfg(feature = "network-isolation")]
    asr::model_manager::assert_network_isolation();

    let show_settings = should_show_settings_on_launch();

    match instance::acquire(show_settings) {
        Ok(None) => return,
        Ok(Some(_guard)) => {}
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }

    let args: Vec<String> = std::env::args().collect();
    if let Some(idx) = args.iter().position(|a| a == "--debug-record") {
        if let Some(secs) = args.get(idx + 1).and_then(|s| s.parse().ok()) {
            if let Err(e) = run_debug_record_cli(secs) {
                eprintln!("debug record failed: {e:#}");
                std::process::exit(1);
            }
            return;
        }
    }

    let saved_config = crate::config::load_config().unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config: Mutex::new(saved_config.clone()),
        })
        .setup(move |app| {
            instance::register_app_handle(app.handle().clone());
            setup_tray(app.handle())?;

            if saved_config.onboarding_complete {
                match init_pipeline(app.handle(), saved_config.clone()) {
                    Ok(p) => {
                        *pipeline_slot().lock() = Some(p);
                    }
                    Err(e) => {
                        log::error!("pipeline init failed: {e:#} — open settings to download model");
                    }
                }
            }

            // Always show settings when launched from the app menu / desktop icon.
            open_settings(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            list_audio_devices,
            list_models,
            download_model_cmd,
            get_permissions,
            test_injection,
            debug_record_wav,
            get_asr_status,
            run_benchmark,
            mic_test_transcribe,
            complete_onboarding,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            RunEvent::Ready { .. } => {
                if instance::show_settings_requested() {
                    open_settings(&app);
                }
            }
            RunEvent::WindowEvent { label, event, .. } => {
                if label == "main" {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                }
            }
            _ => {
                if instance::show_settings_requested() {
                    open_settings(&app);
                }
            }
        });
}

fn run_debug_record_cli(seconds: f32) -> Result<()> {
    let samples = AudioCapture::record_for_seconds(None, seconds)?;
    std::fs::create_dir_all(crate::config::data_dir())?;
    let path = crate::config::data_dir().join("cli-debug.wav");
    save_wav(&path, &samples)?;
    println!("Saved {} samples to {}", samples.len(), path.display());
    Ok(())
}
