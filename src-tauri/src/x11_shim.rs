//! Prevent fatal X11 aborts when a global hotkey is already grabbed by another client.

use std::sync::atomic::{AtomicBool, Ordering};

pub static HOTKEY_GRAB_FAILED: AtomicBool = AtomicBool::new(false);

static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn install_grab_error_handler() {
    if HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
        unsafe {
            (xlib.XSetErrorHandler)(Some(grab_error_handler));
        }
    }
}

pub fn reset_grab_failed_flag() {
    HOTKEY_GRAB_FAILED.store(false, Ordering::SeqCst);
}

pub fn grab_failed() -> bool {
    HOTKEY_GRAB_FAILED.load(Ordering::SeqCst)
}

extern "C" fn grab_error_handler(
    _display: *mut x11_dl::xlib::_XDisplay,
    event: *mut x11_dl::xlib::XErrorEvent,
) -> i32 {
    unsafe {
        let e = &*event;
        // Request 33 = X_GrabKey; BadAccess = combination already grabbed.
        if e.request_code == 33 && e.error_code == x11_dl::xlib::BadAccess as u8 {
            HOTKEY_GRAB_FAILED.store(true, Ordering::SeqCst);
            return 1;
        }
    }
    0
}
