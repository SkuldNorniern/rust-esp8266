#include <Arduino.h>

extern "C" {
uint64_t hello_sum(uint64_t a, uint32_t b, uint64_t c, uint32_t d, uint64_t e, uint32_t f);
void hello_greet(void);

void hello_write(const uint8_t *data, size_t len) { Serial.write(data, len); }
}

void setup() {
    Serial.begin(115200);
    delay(500);
    hello_greet();
    uint64_t want = (1ull << 40) + 2 + (3ull << 33) + 4 + 5 + 6;
    uint64_t got = hello_sum(1ull << 40, 2, 3ull << 33, 4, 5, 6);
    Serial.printf("sum %s\n", got == want ? "ok" : "WRONG");
}

void loop() {}
