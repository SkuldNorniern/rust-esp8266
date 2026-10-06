//! the usb serial port. `Serial` takes `write!`.

use core::fmt;

use crate::sys;

pub fn begin(baud: u32) {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_serial_begin(baud) }
}

pub fn write(data: &[u8]) {
    // SAFETY: the pointer and length come from one live slice.
    unsafe { sys::esp8266_serial_write(data.as_ptr(), data.len()) };
}

/// the next byte received, if any.
#[must_use]
pub fn read() -> Option<u8> {
    // SAFETY: no arguments.
    u8::try_from(unsafe { sys::esp8266_serial_read() }).ok()
}

pub struct Serial;

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(s.as_bytes());
        Ok(())
    }
}
