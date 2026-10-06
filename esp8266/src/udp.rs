//! one udp socket. the runtime has one; binding again moves it to another
//! port. what it sends is marked as voice, the wifi class that goes first.

use core::net::{Ipv4Addr, SocketAddrV4};

use crate::sys;

pub struct Udp(());

impl Udp {
    /// listens on `port`. `None` when the stack refuses.
    #[must_use]
    pub fn bind(port: u16) -> Option<Self> {
        // SAFETY: no pointers.
        unsafe { sys::esp8266_udp_begin(port) }.then_some(Self(()))
    }

    /// sends one datagram. gives false when it could not go out.
    pub fn send(&mut self, to: SocketAddrV4, data: &[u8]) -> bool {
        let ip = to.ip().octets();
        // SAFETY: both pointers and the length come from live arrays.
        unsafe { sys::esp8266_udp_send(ip.as_ptr(), to.port(), data.as_ptr(), data.len()) }
    }

    /// the next datagram into `buf`, with its length and sender.
    pub fn receive(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddrV4)> {
        let (mut ip, mut port) = ([0; 4], 0);
        // SAFETY: all pointers come from live locals and `buf`.
        let n = unsafe {
            sys::esp8266_udp_receive(buf.as_mut_ptr(), buf.len(), ip.as_mut_ptr(), &raw mut port)
        };
        let n = usize::try_from(n).ok()?;
        Some((
            n.min(buf.len()),
            SocketAddrV4::new(Ipv4Addr::from(ip), port),
        ))
    }
}
