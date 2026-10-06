//! rtc memory and why the chip started: what a restart keeps, so a
//! firmware can tell what happened before it.

use crate::sys;

/// words the app has, after the runtime's own.
pub const WORDS: usize = 120;

/// reads from word `at`. words past [`WORDS`] are left alone.
pub fn read(at: usize, out: &mut [u32]) {
    let n = out.len().min(WORDS.saturating_sub(at));
    if n == 0 {
        return;
    }
    // SAFETY: at most `n` words go to `out`, which holds `n` or more, and
    // `at + n` stays inside the app's rtc words.
    #[allow(clippy::cast_possible_truncation, reason = "at is under WORDS")]
    unsafe {
        sys::esp8266_rtc_read(at as u32, out.as_mut_ptr(), n);
    }
}

/// writes from word `at`. words past [`WORDS`] are dropped.
pub fn write(at: usize, data: &[u32]) {
    let n = data.len().min(WORDS.saturating_sub(at));
    if n == 0 {
        return;
    }
    // SAFETY: as in `read`, `n` words from a live slice, inside the app's
    // rtc words.
    #[allow(clippy::cast_possible_truncation, reason = "at is under WORDS")]
    unsafe {
        sys::esp8266_rtc_write(at as u32, data.as_ptr(), n);
    }
}

/// why the chip last started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    PowerOn,
    /// the hardware watchdog: the loop hung with interrupts off.
    HardwareWatchdog,
    /// a crash, with the exception's cause, pc and address.
    Exception {
        cause: u32,
        pc: u32,
        address: u32,
    },
    /// the software watchdog: the loop never yielded.
    SoftwareWatchdog,
    /// a restart asked for.
    Restart,
    DeepSleep,
    /// the reset pin.
    Reset,
    Other(u32),
}

#[must_use]
pub fn start() -> Start {
    let mut r = [0; 5];
    // SAFETY: the runtime writes five words.
    unsafe { sys::esp8266_reset_info(r.as_mut_ptr()) };
    match r[0] {
        0 => Start::PowerOn,
        1 => Start::HardwareWatchdog,
        2 => Start::Exception {
            cause: r[1],
            pc: r[2],
            address: r[3],
        },
        3 => Start::SoftwareWatchdog,
        4 => Start::Restart,
        5 => Start::DeepSleep,
        6 => Start::Reset,
        n => Start::Other(n),
    }
}
