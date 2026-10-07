//! a new firmware over wifi, pulled from a host over tcp and written into
//! the free flash. the next start runs it.

use core::net::SocketAddrV4;

use crate::sys;

/// why an update did not take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failed {
    /// the host never answered on its port.
    NoHost,
    /// the transfer stopped coming before the end.
    Stalled,
    /// the Arduino core's `Updater` said no: too big, a flash write
    /// failed, or the md5 did not match. its error code.
    Updater(i32),
}

/// pulls `size` bytes from `host` and checks them against `md5`. blocks
/// until done, some seconds. `Ok` means the new firmware runs from the
/// next start: restart then.
///
/// # Errors
///
/// when the host is not there, the transfer stalls, or the image is bad.
pub fn pull(host: SocketAddrV4, size: u32, md5: &[u8; 16]) -> Result<(), Failed> {
    let mut hex = [0_u8; 33];
    for (i, b) in md5.iter().enumerate() {
        let digit = |n: u8| if n < 10 { b'0' + n } else { b'a' + n - 10 };
        hex[2 * i] = digit(b >> 4);
        hex[2 * i + 1] = digit(b & 0x0F);
    }
    // the core's IPAddress takes the address with its first byte lowest
    let ip = u32::from_le_bytes(host.ip().octets());
    // SAFETY: `hex` is a live, nul terminated buffer for the whole call.
    match unsafe { sys::esp8266_update(ip, host.port(), size, hex.as_ptr()) } {
        0 => Ok(()),
        100 => Err(Failed::NoHost),
        101 => Err(Failed::Stalled),
        e => Err(Failed::Updater(e)),
    }
}
