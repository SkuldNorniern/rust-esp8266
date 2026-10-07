//! the clock and waiting.

use crate::sys;

/// µs since boot, never wrapping.
#[must_use]
pub fn micros() -> u64 {
    // SAFETY: no arguments, no state.
    unsafe { sys::esp8266_micros() }
}

/// ms since boot, wrapping after 49 days.
#[must_use]
pub fn millis() -> u32 {
    // SAFETY: no arguments, no state.
    unsafe { sys::esp8266_millis() }
}

/// waits `ms`, letting the radio run meanwhile.
pub fn delay(ms: u32) {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_delay(ms) }
}

/// lets the radio run once.
pub fn yield_now() {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_yield() }
}

/// bytes of the loop's 4 KiB stack never used since start: how close it
/// came to overflowing.
#[must_use]
pub fn stack_untouched() -> u32 {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_stack_untouched() }
}

/// restarts the chip.
pub fn restart() -> ! {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_restart() };
    // the restart takes a moment to happen
    loop {
        yield_now();
    }
}
