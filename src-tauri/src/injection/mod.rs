use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionStatus {
    pub granted: bool,
    pub fix_instructions: String,
}

pub trait TextInjector: Send + Sync {
    fn inject(&self, text: &str) -> Result<()>;
    fn inject_via_clipboard(&self, text: &str) -> Result<()>;
    fn check_permissions(&self) -> PermissionStatus;
}

pub fn platform_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "windows")]
    {
        return Box::new(windows::WindowsInjector);
    }
    #[cfg(target_os = "macos")]
    {
        return Box::new(macos::MacOsInjector);
    }
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            return Box::new(linux_wayland::LinuxWaylandInjector);
        }
        return Box::new(linux_x11::LinuxX11Injector);
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        struct NoopInjector;
        impl TextInjector for NoopInjector {
            fn inject(&self, text: &str) -> Result<()> {
                log::warn!("text injection not supported on this platform: {text}");
                Ok(())
            }
            fn inject_via_clipboard(&self, _text: &str) -> Result<()> {
                Ok(())
            }
            fn check_permissions(&self) -> PermissionStatus {
                PermissionStatus {
                    granted: false,
                    fix_instructions: "Platform not supported".into(),
                }
            }
        }
        Box::new(NoopInjector)
    }
}

pub fn inject_with_fallback(injector: &dyn TextInjector, text: &str) -> Result<()> {
    match injector.inject(text) {
        Ok(()) => {
            log::info!("text injected via primary method");
            Ok(())
        }
        Err(primary_err) => {
            log::warn!("primary injection failed: {primary_err:#}, trying clipboard");
            injector.inject_via_clipboard(text)
        }
    }
}

#[cfg(target_os = "linux")]
pub fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var("WAYLAND_DISPLAY").is_ok()
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux_x11;

#[cfg(target_os = "linux")]
mod linux_wayland;
