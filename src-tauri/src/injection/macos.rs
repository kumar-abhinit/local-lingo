use super::{PermissionStatus, TextInjector};
use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use std::thread;
use std::time::Duration;

pub struct MacOsInjector;

impl TextInjector for MacOsInjector {
    fn inject(&self, text: &str) -> Result<()> {
        if !Self::is_trusted() {
            return Err(anyhow!("Accessibility permission not granted"));
        }
        // Unicode typing via clipboard+paste is reliable on macOS
        self.inject_via_clipboard(text)
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new().context("clipboard unavailable")?;
        clipboard.set_text(text)?;
        thread::sleep(Duration::from_millis(50));
        Self::simulate_cmd_v()
    }

    fn check_permissions(&self) -> PermissionStatus {
        if Self::is_trusted() {
            PermissionStatus {
                granted: true,
                fix_instructions: String::new(),
            }
        } else {
            PermissionStatus {
                granted: false,
                fix_instructions: "Enable LocalLingo in System Settings → Privacy & Security → Accessibility".into(),
            }
        }
    }
}

impl MacOsInjector {
    fn is_trusted() -> bool {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        unsafe { AXIsProcessTrusted() }
    }

    fn simulate_cmd_v() -> Result<()> {
        use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
        let key_v: u16 = 9;

        let down = CGEvent::new_keyboard_event(CGEventTapLocation::HID, key_v, true)
            .map_err(|_| anyhow!("failed to create key down event"))?;
        down.set_flags(CGEventFlags::CGEventFlagCommand);
        down.post(CGEventTapLocation::HID);

        thread::sleep(Duration::from_millis(20));

        let up = CGEvent::new_keyboard_event(CGEventTapLocation::HID, key_v, false)
            .map_err(|_| anyhow!("failed to create key up event"))?;
        up.set_flags(CGEventFlags::CGEventFlagCommand);
        up.post(CGEventTapLocation::HID);

        Ok(())
    }
}
