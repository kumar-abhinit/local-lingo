//! Single-instance guard: second launch focuses the running app instead of starting anew.

use crate::config;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static SHOW_SETTINGS_REQUESTED: AtomicBool = AtomicBool::new(false);
static APP_HANDLE: OnceLock<Mutex<Option<tauri::AppHandle>>> = OnceLock::new();

pub fn register_app_handle(app: tauri::AppHandle) {
    APP_HANDLE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .replace(app);
}

pub fn show_settings_requested() -> bool {
    SHOW_SETTINGS_REQUESTED.swap(false, Ordering::SeqCst)
}

fn lock_path() -> PathBuf {
    config::data_dir().join("local-lingo.lock")
}

pub struct InstanceGuard {
    _file: File,
}

#[cfg(unix)]
fn process_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(not(unix))]
fn process_alive(_pid: i32) -> bool {
    false
}

#[cfg(unix)]
fn request_show_settings(pid: i32) -> Result<()> {
    const SIGUSR1: libc::c_int = 10;
    if unsafe { libc::kill(pid, SIGUSR1) } == 0 {
        Ok(())
    } else {
        Err(anyhow!("failed to signal running instance (pid {pid})"))
    }
}

#[cfg(not(unix))]
fn request_show_settings(_pid: i32) -> Result<()> {
    Err(anyhow!("single-instance focus is only supported on Unix"))
}

#[cfg(unix)]
pub fn install_show_settings_handler() {
    use std::sync::Once;

    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        const SIGUSR1: libc::c_int = 10;
        unsafe {
            libc::signal(
                SIGUSR1,
                show_settings_signal_handler as libc::sighandler_t,
            );
        }
    });
}

#[cfg(not(unix))]
pub fn install_show_settings_handler() {}

#[cfg(unix)]
extern "C" fn show_settings_signal_handler(_sig: libc::c_int) {
    SHOW_SETTINGS_REQUESTED.store(true, Ordering::SeqCst);
}

/// Returns `Ok(None)` if another instance was focused and this process should exit.
/// Returns `Ok(Some(guard))` if this process should continue starting.
#[cfg(not(unix))]
pub fn acquire(show_settings: bool) -> Result<Option<InstanceGuard>> {
    let _ = show_settings;
    install_show_settings_handler();
    std::fs::create_dir_all(config::data_dir())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(lock_path())?;
    writeln!(file, "{}", std::process::id())?;
    file.sync_all()?;
    Ok(Some(InstanceGuard { _file: file }))
}

#[cfg(unix)]
pub fn acquire(show_settings: bool) -> Result<Option<InstanceGuard>> {
    install_show_settings_handler();

    std::fs::create_dir_all(config::data_dir())?;
    let path = lock_path();

    if let Ok(mut existing) = File::open(&path) {
        let mut buf = String::new();
        existing.read_to_string(&mut buf)?;
        if let Ok(pid) = buf.trim().parse::<i32>() {
            if process_alive(pid) {
                if show_settings {
                    request_show_settings(pid)?;
                }
                return Ok(None);
            }
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();
        let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            // Another instance holds the lock; try to focus it.
            if show_settings {
                if let Ok(mut existing) = File::open(&path) {
                    let mut buf = String::new();
                    existing.read_to_string(&mut buf)?;
                    if let Ok(pid) = buf.trim().parse::<i32>() {
                        if process_alive(pid) {
                            request_show_settings(pid)?;
                            return Ok(None);
                        }
                    }
                }
            }
            return Err(anyhow!(
                "LocalLingo is already running — use the tray icon or app menu entry to open Settings"
            ));
        }
    }

    writeln!(file, "{}", std::process::id())?;
    file.sync_all()?;

    Ok(Some(InstanceGuard { _file: file }))
}
