use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

pub const TARGET_SAMPLE_RATE: u32 = 16_000;
pub const DEFAULT_TRAILING_SILENCE_MS: u64 = 800;
pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+Space";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyMode {
    PushToTalk,
    Toggle,
}

impl Default for HotkeyMode {
    fn default() -> Self {
        Self::PushToTalk
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Low,
    Medium,
    High,
}

impl Default for ModelTier {
    fn default() -> Self {
        Self::High
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkey: String,
    pub hotkey_mode: HotkeyMode,
    pub mic_device: Option<String>,
    pub model_tier: ModelTier,
    pub model_path: Option<String>,
    pub trailing_silence_ms: u64,
    pub onboarding_complete: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.to_string(),
            hotkey_mode: HotkeyMode::PushToTalk,
            mic_device: None,
            model_tier: ModelTier::High,
            model_path: None,
            trailing_silence_ms: DEFAULT_TRAILING_SILENCE_MS,
            onboarding_complete: false,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("local-lingo")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("local-lingo")
}

pub fn models_dir() -> PathBuf {
    data_dir().join("models")
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let path = config_path();
    if !path.exists() {
        let config = AppConfig::default();
        save_config(&config)?;
        return Ok(config);
    }
    let contents = fs::read_to_string(&path)?;
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let contents = toml::to_string_pretty(config)?;
    fs::write(config_path(), contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = AppConfig::default();
        assert_eq!(config.hotkey, DEFAULT_HOTKEY);
        assert_eq!(config.hotkey_mode, HotkeyMode::PushToTalk);
        assert!(!config.onboarding_complete);
    }
}
