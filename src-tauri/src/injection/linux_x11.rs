use super::{PermissionStatus, TextInjector};
use anyhow::{Context, Result};
use arboard::Clipboard;
use std::thread;
use std::time::Duration;

pub struct LinuxX11Injector;

impl TextInjector for LinuxX11Injector {
    fn inject(&self, text: &str) -> Result<()> {
        self.inject_via_clipboard(text)
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new().context("clipboard unavailable")?;
        clipboard.set_text(text)?;
        thread::sleep(Duration::from_millis(80));
        // Text is on clipboard; user or compositor may need Ctrl+V
        log::info!("text copied to clipboard for X11 paste (Ctrl+V)");
        Ok(())
    }

    fn check_permissions(&self) -> PermissionStatus {
        PermissionStatus {
            granted: true,
            fix_instructions: String::new(),
        }
    }
}
