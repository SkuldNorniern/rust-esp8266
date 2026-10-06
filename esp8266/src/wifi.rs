//! the station. the SDK keeps the last network in its own flash, so a
//! board that joined once, under any firmware, joins again by itself.
//! 802.11n at full power, reconnecting by itself; after a restart it goes
//! straight to the access point and channel it was on, without a scan.

use core::net::Ipv4Addr;

use crate::sys;

/// how the radio sleeps between beacons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sleep {
    /// always listening: lowest latency, most current.
    None = 0,
    /// off between beacons: data waits for the next one.
    Modem = 1,
    Light = 2,
}

/// longest name or password the SDK takes, bytes.
const MOST: usize = 64;

/// a NUL ended copy of `s`, cut at `MOST`.
fn c_string(s: &str) -> [u8; MOST + 1] {
    let mut out = [0; MOST + 1];
    let n = s.len().min(MOST);
    out[..n].copy_from_slice(&s.as_bytes()[..n]);
    out
}

/// starts the station on the kept network.
pub fn begin(hostname: &str, sleep: Sleep) {
    let name = c_string(hostname);
    // SAFETY: `name` is NUL ended and lives through the call.
    unsafe { sys::esp8266_wifi_begin(name.as_ptr(), sleep as u8) }
}

/// how the radio sleeps from now on.
pub fn sleep(sleep: Sleep) {
    // SAFETY: no pointers.
    unsafe { sys::esp8266_wifi_sleep(sleep as u8) }
}

/// joins another network and keeps it.
pub fn join(ssid: &str, password: &str) {
    let (s, p) = (c_string(ssid), c_string(password));
    // SAFETY: both are NUL ended and live through the call.
    unsafe { sys::esp8266_wifi_join(s.as_ptr(), p.as_ptr()) }
}

#[must_use]
pub fn connected() -> bool {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_wifi_connected() }
}

/// the SDK's reason the link last went down (201 no network, 202 bad
/// password, 200 beacon lost, ...), `None` before it ever did.
#[must_use]
pub fn last_drop() -> Option<u8> {
    // SAFETY: no arguments.
    let r = unsafe { sys::esp8266_wifi_last_reason() };
    (r != 0).then_some(r)
}

/// the core's link status: 0 idle, 1 network not found, 3 connected, 4
/// failed, 6 bad password, 7 down.
#[must_use]
pub fn status() -> u8 {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_wifi_status() }
}

/// the channel it is on, 0 while not connected.
#[must_use]
pub fn channel() -> u8 {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_wifi_channel() }
}

/// signal strength, dBm.
#[must_use]
pub fn rssi() -> i8 {
    // SAFETY: no arguments.
    unsafe { sys::esp8266_wifi_rssi() }
}

#[must_use]
pub fn mac() -> [u8; 6] {
    let mut out = [0; 6];
    // SAFETY: `out` holds the 6 bytes written.
    unsafe { sys::esp8266_wifi_mac(out.as_mut_ptr()) };
    out
}

/// the kept network's name, in `buf`.
pub fn ssid(buf: &mut [u8; 32]) -> &str {
    // SAFETY: the pointer and length come from `buf`.
    let n = unsafe { sys::esp8266_wifi_ssid(buf.as_mut_ptr(), buf.len()) };
    core::str::from_utf8(&buf[..n.min(32)]).unwrap_or("")
}

/// own address and the network mask, once connected.
#[must_use]
pub fn address() -> Option<(Ipv4Addr, Ipv4Addr)> {
    if !connected() {
        return None;
    }
    let (mut ip, mut mask) = ([0; 4], [0; 4]);
    // SAFETY: both hold the 4 bytes written.
    unsafe { sys::esp8266_wifi_address(ip.as_mut_ptr(), mask.as_mut_ptr()) };
    Some((ip.into(), mask.into()))
}

/// everyone on this network.
#[must_use]
pub fn broadcast() -> Option<Ipv4Addr> {
    let (ip, mask) = address()?;
    Some(Ipv4Addr::from_bits(ip.to_bits() | !mask.to_bits()))
}
