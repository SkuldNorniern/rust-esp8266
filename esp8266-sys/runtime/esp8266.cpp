// the Arduino esp8266 core as plain C functions, for the `esp8266` crate.
// the radio and the SDK are only reachable from C++, so this file is the
// whole C side; the app is Rust and defines setup() and loop() itself.

#include <Arduino.h>
#include <EEPROM.h>
#include <ESP8266WiFi.h>
#include <WiFiUdp.h>
#include <Wire.h>

namespace {
WiFiUDP udp;
}

extern "C" {

// time

uint64_t esp8266_micros(void) { return micros64(); }
uint32_t esp8266_millis(void) { return millis(); }
void esp8266_delay(uint32_t ms) { delay(ms); }
void esp8266_yield(void) { yield(); }
void esp8266_restart(void) { ESP.restart(); }

// serial

void esp8266_serial_begin(uint32_t baud) { Serial.begin(baud); }
size_t esp8266_serial_write(const uint8_t *data, size_t len) { return Serial.write(data, len); }
int esp8266_serial_read(void) { return Serial.read(); }

// pins

void esp8266_pin_mode(uint8_t pin, uint8_t mode) { pinMode(pin, mode); }
void esp8266_digital_write(uint8_t pin, bool high) { digitalWrite(pin, high ? HIGH : LOW); }
bool esp8266_digital_read(uint8_t pin) { return digitalRead(pin) == HIGH; }
uint16_t esp8266_analog_read(void) { return (uint16_t)analogRead(A0); }

// i2c

void esp8266_i2c_begin(uint8_t sda, uint8_t scl, uint32_t hz) {
    Wire.begin(sda, scl);
    Wire.setClock(hz);
}

bool esp8266_i2c_write(uint8_t address, const uint8_t *data, size_t len) {
    Wire.beginTransmission(address);
    Wire.write(data, len);
    return Wire.endTransmission() == 0;
}

// writes `write` then reads `read_len` bytes with a repeated start.
bool esp8266_i2c_write_read(uint8_t address, const uint8_t *write, size_t write_len,
                            uint8_t *read, size_t read_len) {
    Wire.beginTransmission(address);
    Wire.write(write, write_len);
    if (Wire.endTransmission(false) != 0) return false;
    if (Wire.requestFrom(address, (uint8_t)read_len) != read_len) return false;
    for (size_t i = 0; i < read_len; i++) read[i] = (uint8_t)Wire.read();
    return true;
}

// wifi

// sleep: 0 none, 1 modem, 2 light.
void esp8266_wifi_begin(const char *hostname, uint8_t sleep) {
    WiFi.persistent(true);
    WiFi.mode(WIFI_STA);
    WiFi.hostname(hostname);
    WiFi.setSleepMode(sleep == 2 ? WIFI_LIGHT_SLEEP : sleep == 1 ? WIFI_MODEM_SLEEP : WIFI_NONE_SLEEP);
    // the network the SDK kept from before
    WiFi.begin();
}

// sleep: 0 none, 1 modem, 2 light, while running.
void esp8266_wifi_sleep(uint8_t sleep) {
    WiFi.setSleepMode(sleep == 2 ? WIFI_LIGHT_SLEEP : sleep == 1 ? WIFI_MODEM_SLEEP : WIFI_NONE_SLEEP);
}

void esp8266_wifi_join(const char *ssid, const char *password) {
    WiFi.persistent(true);
    WiFi.begin(ssid, password);
}

bool esp8266_wifi_connected(void) { return WiFi.status() == WL_CONNECTED; }
int8_t esp8266_wifi_rssi(void) { return (int8_t)WiFi.RSSI(); }
void esp8266_wifi_mac(uint8_t *out) { WiFi.macAddress(out); }

// copies the kept network name into `out`, gives its length.
size_t esp8266_wifi_ssid(uint8_t *out, size_t len) {
    String s = WiFi.SSID();
    size_t n = s.length() < len ? s.length() : len;
    memcpy(out, s.c_str(), n);
    return n;
}

void esp8266_wifi_address(uint8_t *ip, uint8_t *mask) {
    IPAddress a = WiFi.localIP(), m = WiFi.subnetMask();
    for (int i = 0; i < 4; i++) {
        ip[i] = a[i];
        mask[i] = m[i];
    }
}

// udp

bool esp8266_udp_begin(uint16_t port) { return udp.begin(port) == 1; }

bool esp8266_udp_send(const uint8_t *ip, uint16_t port, const uint8_t *data, size_t len) {
    if (!udp.beginPacket(IPAddress(ip[0], ip[1], ip[2], ip[3]), port)) return false;
    udp.write(data, len);
    return udp.endPacket() == 1;
}

// the next datagram into `out`: gives its length, or -1 when none waits.
int esp8266_udp_receive(uint8_t *out, size_t len, uint8_t *ip, uint16_t *port) {
    if (udp.parsePacket() <= 0) return -1;
    IPAddress from = udp.remoteIP();
    for (int i = 0; i < 4; i++) ip[i] = from[i];
    *port = udp.remotePort();
    return udp.read(out, len);
}

// flash, as the core's EEPROM emulation

void esp8266_flash_begin(size_t size) { EEPROM.begin(size); }

void esp8266_flash_read(size_t at, uint8_t *out, size_t len) {
    for (size_t i = 0; i < len; i++) out[i] = EEPROM.read((int)(at + i));
}

bool esp8266_flash_write(size_t at, const uint8_t *data, size_t len) {
    for (size_t i = 0; i < len; i++) EEPROM.write((int)(at + i), data[i]);
    return EEPROM.commit();
}

}
