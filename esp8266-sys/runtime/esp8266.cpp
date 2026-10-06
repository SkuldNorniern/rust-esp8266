// the Arduino esp8266 core as plain C functions, for the `esp8266` crate.
// the radio and the SDK are only reachable from C++, so this file is the
// whole C side; the app is Rust and defines setup() and loop() itself.

#include <Arduino.h>
#include <EEPROM.h>
#include <ESP8266WiFi.h>
#include <WiFiUdp.h>
#include <Wire.h>

extern "C" {
#include <lwip/udp.h>
extern struct udp_pcb *udp_pcbs;
}

namespace {
WiFiUDP udp;

// voice, the highest wifi access class: sent first, waits least.
const uint8_t VOICE_TOS = 0xC0;

// where the last join ended up, kept in RTC memory: a restart joins that
// access point on that channel straight away instead of scanning.
const uint32_t FAST_MAGIC = 0xA17AF00Du;
struct FastJoin {
    uint32_t magic;
    uint8_t bssid[6];
    uint8_t channel;
    uint8_t spare;
};

WiFiEventHandler got_ip;
WiFiEventHandler lost;
// why the link last went down, 0 for not yet.
volatile uint8_t last_reason = 0;
// a fast join that failed: the next look joins the plain way.
volatile bool fast_failed = false;
bool fast_trying = false;

void remember_join() {
    FastJoin f = {FAST_MAGIC, {0}, (uint8_t)WiFi.channel(), 0};
    memcpy(f.bssid, WiFi.BSSID(), 6);
    ESP.rtcUserMemoryWrite(0, (uint32_t *)&f, sizeof f);
}

void forget_join() {
    FastJoin f = {0, {0}, 0, 0};
    ESP.rtcUserMemoryWrite(0, (uint32_t *)&f, sizeof f);
}
}

extern "C" {

// time

uint64_t esp8266_micros(void) { return micros64(); }
uint32_t esp8266_millis(void) { return millis(); }
void esp8266_delay(uint32_t ms) { delay(ms); }
void esp8266_yield(void) { yield(); }
void esp8266_restart(void) { ESP.restart(); }

// why the chip last started: reason, exception cause, pc, address, depc.
void esp8266_reset_info(uint32_t *out) {
    const rst_info *r = ESP.getResetInfoPtr();
    out[0] = r->reason;
    out[1] = r->exccause;
    out[2] = r->epc1;
    out[3] = r->excvaddr;
    out[4] = r->depc;
}

// rtc memory, kept over a restart but not a power loss. the first
// RTC_OWN blocks are the runtime's (the fast join), the rest the app's.
const uint32_t RTC_OWN = 8;

void esp8266_rtc_read(uint32_t block, uint32_t *out, size_t words) {
    ESP.rtcUserMemoryRead(RTC_OWN + block, out, words * 4);
}

void esp8266_rtc_write(uint32_t block, const uint32_t *data, size_t words) {
    ESP.rtcUserMemoryWrite(RTC_OWN + block, (uint32_t *)data, words * 4);
}

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
    // n is the fastest the chip has: the least air time per packet
    WiFi.setPhyMode(WIFI_PHY_MODE_11N);
    WiFi.setOutputPower(20.5f);
    WiFi.setAutoReconnect(true);
    WiFi.setSleepMode(sleep == 2 ? WIFI_LIGHT_SLEEP : sleep == 1 ? WIFI_MODEM_SLEEP : WIFI_NONE_SLEEP);
    got_ip = WiFi.onStationModeGotIP([](const WiFiEventStationModeGotIP &) {
        fast_trying = false;
        remember_join();
    });
    lost = WiFi.onStationModeDisconnected([](const WiFiEventStationModeDisconnected &e) {
        last_reason = (uint8_t)e.reason;
        if (fast_trying) {
            fast_trying = false;
            fast_failed = true;
        }
    });
    // the network the SDK kept from before, straight to its access point
    // when a restart left where that was
    FastJoin f;
    String ssid = WiFi.SSID();
    if (ESP.rtcUserMemoryRead(0, (uint32_t *)&f, sizeof f) && f.magic == FAST_MAGIC &&
        ssid.length() > 0 && f.channel > 0 && f.channel <= 14) {
        fast_trying = true;
        WiFi.begin(ssid.c_str(), WiFi.psk().c_str(), f.channel, f.bssid);
    } else {
        WiFi.begin();
    }
}

// sleep: 0 none, 1 modem, 2 light, while running.
void esp8266_wifi_sleep(uint8_t sleep) {
    WiFi.setSleepMode(sleep == 2 ? WIFI_LIGHT_SLEEP : sleep == 1 ? WIFI_MODEM_SLEEP : WIFI_NONE_SLEEP);
}

void esp8266_wifi_join(const char *ssid, const char *password) {
    forget_join();
    fast_trying = false;
    WiFi.persistent(true);
    WiFi.begin(ssid, password);
}

bool esp8266_wifi_connected(void) {
    if (fast_failed) {
        // the access point moved or changed channel: scan for it
        fast_failed = false;
        forget_join();
        WiFi.begin();
    }
    return WiFi.status() == WL_CONNECTED;
}

// why the link last went down (the SDK's reason code), 0 for not yet.
uint8_t esp8266_wifi_last_reason(void) { return last_reason; }
uint8_t esp8266_wifi_channel(void) { return (uint8_t)WiFi.channel(); }
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

bool esp8266_udp_begin(uint16_t port) {
    if (udp.begin(port) != 1) return false;
    for (struct udp_pcb *pcb = udp_pcbs; pcb != NULL; pcb = pcb->next) {
        if (pcb->local_port == port) pcb->tos = VOICE_TOS;
    }
    return true;
}

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
