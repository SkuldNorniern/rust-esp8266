//! says hello on serial and blinks the led, all from Rust.

#![no_std]

use core::fmt::Write;

use esp8266::serial::{self, Serial};
use esp8266::{App, pin, time, wifi};

const LED: u8 = 2;

struct Hello {
    on: bool,
    last_ms: u32,
}

impl App for Hello {
    fn setup() -> Self {
        serial::begin(115_200);
        pin::mode(LED, pin::Mode::Output);
        let mac = wifi::mac();
        let _ = writeln!(
            Serial,
            "hello from rust, mac {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}, {} µs up",
            mac[0],
            mac[1],
            mac[2],
            mac[3],
            mac[4],
            mac[5],
            time::micros()
        );
        Self {
            on: false,
            last_ms: 0,
        }
    }

    fn tick(&mut self) {
        let now = time::millis();
        if now.wrapping_sub(self.last_ms) >= 500 {
            self.last_ms = now;
            self.on = !self.on;
            pin::write(LED, !self.on);
        }
        while let Some(b) = serial::read() {
            serial::write(&[b]);
        }
    }
}

esp8266::app!(Hello);
