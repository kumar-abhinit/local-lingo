use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayState {
    Idle,
    Listening,
    Transcribing,
    Error,
}

impl TrayState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "LocalLingo — Ready",
            Self::Listening => "LocalLingo — Listening…",
            Self::Transcribing => "LocalLingo — Transcribing…",
            Self::Error => "LocalLingo — Error",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        self.label()
    }
}

pub fn tray_menu_items() -> Vec<(&'static str, &'static str)> {
    vec![
        ("settings", "Settings…"),
        ("mic_test", "Mic Test"),
        ("quit", "Quit"),
    ]
}
