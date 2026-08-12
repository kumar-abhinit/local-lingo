use super::{PermissionStatus, TextInjector};
use anyhow::{Context, Result};
use arboard::Clipboard;
use std::thread;
use std::time::Duration;

pub struct LinuxWaylandInjector;

impl TextInjector for LinuxWaylandInjector {
    fn inject(&self, text: &str) -> Result<()> {
        match try_xkb_type(text) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::warn!("xkb-type injection failed: {e:#}, falling back to clipboard");
                self.inject_via_clipboard(text)
            }
        }
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new().context("clipboard unavailable")?;
        clipboard.set_text(text)?;
        thread::sleep(Duration::from_millis(80));
        simulate_ctrl_v_wayland()
    }

    fn check_permissions(&self) -> PermissionStatus {
        let uinput_ok = std::path::Path::new("/dev/uinput").exists() && nix_access_uinput();
        if uinput_ok {
            PermissionStatus {
                granted: true,
                fix_instructions: String::new(),
            }
        } else {
            PermissionStatus {
                granted: false,
                fix_instructions: "Add your user to the 'input' group and install udev rule for /dev/uinput. Clipboard paste fallback will be used.".into(),
            }
        }
    }
}

fn try_xkb_type(text: &str) -> Result<()> {
    use std::time::Duration as StdDuration;
    use xkb_type::Keyboard;
    let mut keyboard = Keyboard::new(StdDuration::from_millis(2))
        .context("failed to open uinput device — check input group")?;
    keyboard.type_text(text).context("xkb-type failed")?;
    Ok(())
}

fn simulate_ctrl_v_wayland() -> Result<()> {
    if let Ok(output) = std::process::Command::new("ydotool")
        .args(["key", "29:1", "47:1", "47:0", "29:0"])
        .output()
    {
        if output.status.success() {
            return Ok(());
        }
    }
    log::info!("ydotool unavailable — text is on clipboard, paste with Ctrl+V");
    Ok(())
}

fn nix_access_uinput() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok()
}
