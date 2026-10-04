//! Turns termination requests into an error at the next wait for a renderer, so
//! the renderer's process group and temporary files are cleaned up as on any
//! other failure. Renderers run in their own process group and do not receive
//! the terminal's Ctrl+C themselves.
use anyhow::{Result, bail};
use std::sync::atomic::{AtomicI32, Ordering};

static SIGNAL: AtomicI32 = AtomicI32::new(0);

/// Handles SIGINT, SIGTERM and SIGHUP (console Ctrl+C and Ctrl+Break on
/// Windows). A second request terminates immediately.
pub fn install() {
    #[cfg(unix)]
    unsafe {
        extern "C" fn record(signal: libc::c_int) {
            SIGNAL.store(signal, Ordering::Relaxed);
        }
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = record as extern "C" fn(libc::c_int) as usize;
        // Restart interrupted reads; waits poll for the signal at least every 100 ms.
        action.sa_flags = libc::SA_RESTART | libc::SA_RESETHAND;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            libc::sigaction(signal, &action, std::ptr::null_mut());
        }
    }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{FALSE, TRUE},
            System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler},
        };
        unsafe extern "system" fn record(event: u32) -> windows_sys::core::BOOL {
            if (event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT)
                && SIGNAL.swap(2, Ordering::Relaxed) == 0
            {
                TRUE
            } else {
                FALSE
            }
        }
        SetConsoleCtrlHandler(Some(record), TRUE);
    }
}

/// Fails once a termination request has been received.
pub fn check() -> Result<()> {
    let signal = SIGNAL.load(Ordering::Relaxed);
    if signal != 0 {
        bail!("interrupted (signal {signal})");
    }
    Ok(())
}

/// The conventional exit status after a termination request (128 + signal).
pub fn exit_code() -> Option<u8> {
    match SIGNAL.load(Ordering::Relaxed) {
        0 => None,
        signal => Some(128 + signal as u8),
    }
}
