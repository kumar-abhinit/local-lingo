use crate::config::HotkeyMode;
use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Pressed,
    Released,
}

pub struct HotkeyListener {
    _manager: GlobalHotKeyManager,
    stop_flag: Arc<AtomicBool>,
}

impl HotkeyListener {
    pub fn spawn(hotkey_str: &str, mode: HotkeyMode) -> Result<(Self, Receiver<HotkeyEvent>)> {
        #[cfg(target_os = "linux")]
        {
            crate::x11_shim::install_grab_error_handler();
            crate::x11_shim::reset_grab_failed_flag();
        }

        let (tx, rx) = crossbeam_channel::unbounded();
        let manager = GlobalHotKeyManager::new().map_err(|e| anyhow!("hotkey manager: {e}"))?;
        let hotkey = parse_hotkey(hotkey_str)?;
        manager
            .register(hotkey)
            .map_err(|e| anyhow!("failed to register hotkey '{hotkey_str}': {e}"))?;

        #[cfg(target_os = "linux")]
        {
            // X11 reports GrabKey conflicts asynchronously via the error handler.
            std::thread::sleep(std::time::Duration::from_millis(50));
            if crate::x11_shim::grab_failed() {
                return Err(anyhow!(
                    "hotkey '{hotkey_str}' is already used by another app — change it in Settings (e.g. Ctrl+Alt+Space)"
                ));
            }
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop_flag);

        thread::spawn(move || {
            let receiver = GlobalHotKeyEvent::receiver();
            while !stop_clone.load(Ordering::SeqCst) {
                if let Ok(event) = receiver.try_recv() {
                    if event.state == HotKeyState::Pressed {
                        let _ = tx.send(HotkeyEvent::Pressed);
                        if mode == HotkeyMode::Toggle {
                            // Release events ignored in toggle — pipeline handles toggle
                        }
                    } else if event.state == HotKeyState::Released
                        && mode == HotkeyMode::PushToTalk
                    {
                        let _ = tx.send(HotkeyEvent::Released);
                    }
                }
                thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        Ok((Self { _manager: manager, stop_flag }, rx))
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

fn parse_hotkey(s: &str) -> Result<HotKey> {
    let parts: Vec<&str> = s.split('+').map(str::trim).collect();
    if parts.is_empty() {
        return Err(anyhow!("empty hotkey"));
    }

    let mut mods = Modifiers::empty();
    let mut code = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "cmd" | "command" | "super" | "meta" | "win" => mods |= Modifiers::SUPER,
            key => code = Some(parse_key_code(key)?),
        }
    }

    let code = code.ok_or_else(|| anyhow!("hotkey missing key code"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_key_code(key: &str) -> Result<Code> {
    match key.to_lowercase().as_str() {
        "space" => Ok(Code::Space),
        "enter" | "return" => Ok(Code::Enter),
        "tab" => Ok(Code::Tab),
        "escape" | "esc" => Ok(Code::Escape),
        "f1" => Ok(Code::F1),
        "f2" => Ok(Code::F2),
        "f3" => Ok(Code::F3),
        "f4" => Ok(Code::F4),
        "f5" => Ok(Code::F5),
        "f6" => Ok(Code::F6),
        "f7" => Ok(Code::F7),
        "f8" => Ok(Code::F8),
        "f9" => Ok(Code::F9),
        "f10" => Ok(Code::F10),
        "f11" => Ok(Code::F11),
        "f12" => Ok(Code::F12),
        other if other.len() == 1 => {
            let c = other.chars().next().unwrap();
            key_char_to_code(c)
        }
        other => Err(anyhow!("unsupported key: {other}")),
    }
}

fn key_char_to_code(c: char) -> Result<Code> {
    use Code::*;
    let code = match c.to_ascii_lowercase() {
        'a' => KeyA,
        'b' => KeyB,
        'c' => KeyC,
        'd' => KeyD,
        'e' => KeyE,
        'f' => KeyF,
        'g' => KeyG,
        'h' => KeyH,
        'i' => KeyI,
        'j' => KeyJ,
        'k' => KeyK,
        'l' => KeyL,
        'm' => KeyM,
        'n' => KeyN,
        'o' => KeyO,
        'p' => KeyP,
        'q' => KeyQ,
        'r' => KeyR,
        's' => KeyS,
        't' => KeyT,
        'u' => KeyU,
        'v' => KeyV,
        'w' => KeyW,
        'x' => KeyX,
        'y' => KeyY,
        'z' => KeyZ,
        '0' => Digit0,
        '1' => Digit1,
        '2' => Digit2,
        '3' => Digit3,
        '4' => Digit4,
        '5' => Digit5,
        '6' => Digit6,
        '7' => Digit7,
        '8' => Digit8,
        '9' => Digit9,
        _ => return Err(anyhow!("unsupported character key: {c}")),
    };
    Ok(code)
}

#[allow(dead_code)]
impl FromStr for HotkeyEvent {
    type Err = anyhow::Error;
    fn from_str(_s: &str) -> Result<Self> {
        Err(anyhow!("not implemented"))
    }
}
