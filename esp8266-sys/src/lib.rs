//! raw bindings to `runtime/esp8266.cpp`, the Arduino esp8266 core as
//! plain C functions. the `esp8266` crate is the safe side.
//!
//! every function runs on the Arduino loop only, never from an interrupt.

#![no_std]

unsafe extern "C" {
    pub fn esp8266_micros() -> u64;
    pub fn esp8266_millis() -> u32;
    pub fn esp8266_delay(ms: u32);
    pub fn esp8266_yield();
    pub fn esp8266_restart();

    pub fn esp8266_serial_begin(baud: u32);
    pub fn esp8266_serial_write(data: *const u8, len: usize) -> usize;
    pub fn esp8266_serial_read() -> i32;

    pub fn esp8266_pin_mode(pin: u8, mode: u8);
    pub fn esp8266_digital_write(pin: u8, high: bool);
    pub fn esp8266_digital_read(pin: u8) -> bool;
    pub fn esp8266_analog_read() -> u16;

    pub fn esp8266_i2c_begin(sda: u8, scl: u8, hz: u32);
    pub fn esp8266_i2c_write(address: u8, data: *const u8, len: usize) -> bool;
    pub fn esp8266_i2c_write_read(
        address: u8,
        write: *const u8,
        write_len: usize,
        read: *mut u8,
        read_len: usize,
    ) -> bool;

    pub fn esp8266_wifi_begin(hostname: *const u8, sleep: u8);
    pub fn esp8266_wifi_sleep(sleep: u8);
    pub fn esp8266_wifi_join(ssid: *const u8, password: *const u8);
    pub fn esp8266_wifi_connected() -> bool;
    pub fn esp8266_wifi_rssi() -> i8;
    pub fn esp8266_wifi_mac(out: *mut u8);
    pub fn esp8266_wifi_ssid(out: *mut u8, len: usize) -> usize;
    pub fn esp8266_wifi_address(ip: *mut u8, mask: *mut u8);

    pub fn esp8266_udp_begin(port: u16) -> bool;
    pub fn esp8266_udp_send(ip: *const u8, port: u16, data: *const u8, len: usize) -> bool;
    pub fn esp8266_udp_receive(out: *mut u8, len: usize, ip: *mut u8, port: *mut u16) -> i32;

    pub fn esp8266_flash_begin(size: usize);
    pub fn esp8266_flash_read(at: usize, out: *mut u8, len: usize);
    pub fn esp8266_flash_write(at: usize, data: *const u8, len: usize) -> bool;
}
