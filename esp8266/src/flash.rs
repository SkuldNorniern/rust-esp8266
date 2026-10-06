//! a little flash for settings, through the core's EEPROM emulation: one
//! sector, read into RAM at `begin`, written back on every write.

use crate::sys;

/// most bytes the core gives.
pub const MOST: usize = 4096;

/// takes `size` bytes, at most [`MOST`].
pub fn begin(size: usize) {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_flash_begin(size.min(MOST)) }
}

pub fn read(at: usize, out: &mut [u8]) {
    // SAFETY: the pointer and length come from one live slice.
    unsafe { sys::esp8266_flash_read(at, out.as_mut_ptr(), out.len()) }
}

/// writes and commits. gives false when the flash write failed.
#[must_use]
pub fn write(at: usize, data: &[u8]) -> bool {
    // SAFETY: the pointer and length come from one live slice.
    unsafe { sys::esp8266_flash_write(at, data.as_ptr(), data.len()) }
}
