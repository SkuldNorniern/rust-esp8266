//! gpio and the one analog input.

use crate::sys;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Input = 0,
    Output = 1,
    InputPullup = 2,
}

pub fn mode(pin: u8, mode: Mode) {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_pin_mode(pin, mode as u8) }
}

pub fn write(pin: u8, high: bool) {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_digital_write(pin, high) }
}

#[must_use]
pub fn read(pin: u8) -> bool {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_digital_read(pin) }
}

/// A0, 0..=1023 over 0..1 V.
#[must_use]
pub fn analog() -> u16 {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_analog_read() }
}
