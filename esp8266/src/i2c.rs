//! the i2c bus, as master.

use crate::sys;

/// the part did not answer, or answered short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nack;

/// the bus. there is one; making it again moves it to other pins.
pub struct I2c(());

impl I2c {
    #[must_use]
    pub fn new(sda: u8, scl: u8, hz: u32) -> Self {
        // SAFETY: no pointers.
        unsafe { sys::esp8266_i2c_begin(sda, scl, hz) };
        Self(())
    }

    /// # Errors
    ///
    /// when the part does not answer.
    pub fn write(&mut self, address: u8, data: &[u8]) -> Result<(), Nack> {
        // SAFETY: the pointer and length come from one live slice.
        unsafe { sys::esp8266_i2c_write(address, data.as_ptr(), data.len()) }
            .then_some(())
            .ok_or(Nack)
    }

    /// writes `write`, then fills `read` after a repeated start.
    ///
    /// # Errors
    ///
    /// when the part does not answer or sends less.
    pub fn write_read(&mut self, address: u8, write: &[u8], read: &mut [u8]) -> Result<(), Nack> {
        // SAFETY: both pointers and lengths come from live slices.
        unsafe {
            sys::esp8266_i2c_write_read(
                address,
                write.as_ptr(),
                write.len(),
                read.as_mut_ptr(),
                read.len(),
            )
        }
        .then_some(())
        .ok_or(Nack)
    }
}
