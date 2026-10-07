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

WiFiEventHandler lost;
// why the link last went down, 0 for not yet.
volatile uint8_t last_reason = 0;
// when the link was last seen down, 0 while up.
uint32_t down_since = 0;
// a join that has not joined by then starts over, ms: the SDK can sit
// on a join without ever saying it failed.
const uint32_t JOIN_FOR = 20000;

// joins the kept network by scanning for it, and keeps it plain: a
// config held to one access point and channel holds every later join
// to them.
void join_plain() {
    String ssid = WiFi.SSID();
    String psk = WiFi.psk();
    if (ssid.length() == 0) return;
    WiFi.persistent(true);
    WiFi.begin(ssid.c_str(), psk.c_str());
}
}

extern "C" {

// time

uint64_t esp8266_micros(void) { return micros64(); }
uint32_t esp8266_millis(void) { return millis(); }
void esp8266_delay(uint32_t ms) { delay(ms); }
void esp8266_yield(void) { yield(); }
// leaves the access point first: one that still holds the old link can
// keep the next join waiting. the SDK call keeps the saved network.
void esp8266_restart(void) {
    wifi_station_disconnect();
    delay(20);
    ESP.restart();
}

// bytes of the loop's stack never used yet: its low-water mark.
uint32_t esp8266_stack_untouched(void) { return ESP.getFreeContStack(); }

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
// RTC_OWN blocks are kept for the runtime, the rest are the app's.
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
    // SlimeVR's 17.5 dBm: full 20.5 draws more than a battery's regulator
    // holds, and the link breaks off USB power
    WiFi.setOutputPower(17.5f);
    WiFi.setAutoReconnect(true);
    WiFi.setSleepMode(sleep == 2 ? WIFI_LIGHT_SLEEP : sleep == 1 ? WIFI_MODEM_SLEEP : WIFI_NONE_SLEEP);
    WiFi.setAutoConnect(true);
    lost = WiFi.onStationModeDisconnected(
        [](const WiFiEventStationModeDisconnected &e) { last_reason = (uint8_t)e.reason; });
    // the SDK joins the kept network by itself from boot, faster than any
    // join started here, and a second join started meanwhile can stall it.
    // only a kept config held to one access point is written plain again.
    station_config kept;
    if (wifi_station_get_config_default(&kept) && kept.bssid_set) {
        join_plain();
    } else if (wifi_station_get_connect_status() == STATION_IDLE) {
        // settings above can leave it idle: join, without a second start
        WiFi.begin();
    }
    down_since = millis() | 1;
}

// transmit power, quarter dBm, 0 to 82, while running.
void esp8266_wifi_power(uint8_t quarter_dbm) { WiFi.setOutputPower(quarter_dbm / 4.0f); }

// sleep: 0 none, 1 modem, 2 light, while running.
void esp8266_wifi_sleep(uint8_t sleep) {
    WiFi.setSleepMode(sleep == 2 ? WIFI_LIGHT_SLEEP : sleep == 1 ? WIFI_MODEM_SLEEP : WIFI_NONE_SLEEP);
}

void esp8266_wifi_join(const char *ssid, const char *password) {
    WiFi.persistent(true);
    WiFi.begin(ssid, password);
    down_since = millis() | 1;
}

bool esp8266_wifi_connected(void) {
    if (WiFi.status() == WL_CONNECTED) {
        down_since = 0;
        return true;
    }
    uint32_t now = millis();
    if (down_since == 0) down_since = now | 1;
    uint32_t down = now - down_since;
    // idle is not trying at all; past JOIN_FOR the join stalled or the
    // access point is gone: start over
    bool idle = wifi_station_get_connect_status() == STATION_IDLE;
    if (down > JOIN_FOR || (idle && down > 1000)) {
        join_plain();
        down_since = now | 1;
    }
    return false;
}

// why the link last went down (the SDK's reason code), 0 for not yet.
uint8_t esp8266_wifi_last_reason(void) { return last_reason; }
// the core's link status: 0 idle, 1 network not found, 3 connected, 4 failed, 6 bad password, 7 down.
uint8_t esp8266_wifi_status(void) { return (uint8_t)WiFi.status(); }
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
