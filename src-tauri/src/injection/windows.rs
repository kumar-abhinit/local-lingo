use super::{PermissionStatus, TextInjector};
use anyhow::{anyhow, Context, Result};
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE, KEYEVENTF_KEYUP,
    VK_CONTROL, VK_V,
};

pub struct WindowsInjector;

impl TextInjector for WindowsInjector {
    fn inject(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            send_unicode_char(ch)?;
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<()> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new().context("clipboard unavailable")?;
        clipboard.set_text(text).context("failed to set clipboard")?;
        thread::sleep(Duration::from_millis(50));
        send_ctrl_v()
    }

    fn check_permissions(&self) -> PermissionStatus {
        PermissionStatus {
            granted: true,
            fix_instructions: String::new(),
        }
    }
}

fn send_unicode_char(ch: char) -> Result<()> {
    let code = ch as u16;
    let inputs = [
        make_unicode_input(code, false),
        make_unicode_input(code, true),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(anyhow!("SendInput failed"));
    }
    Ok(())
}

fn make_unicode_input(code: u16, key_up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: Default::default(),
                wScan: code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_ctrl_v() -> Result<()> {
    let ctrl_down = key_input(VK_CONTROL.0 as u16, false);
    let v_down = key_input(VK_V.0 as u16, false);
    let v_up = key_input(VK_V.0 as u16, true);
    let ctrl_up = key_input(VK_CONTROL.0 as u16, true);
    let inputs = [ctrl_down, v_down, v_up, ctrl_up];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(anyhow!("SendInput Ctrl+V failed"));
    }
    Ok(())
}

fn key_input(vk: u16, key_up: bool) -> INPUT {
    let mut flags = Default::default();
    if key_up {
        flags = KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
