# rust-esp8266

Rust apps on the ESP8266, on top of the Arduino core, built with PlatformIO.

The esp Rust toolchain still has the `xtensa-esp8266-none-elf` target and builds call0 code, the same convention lx106 gcc uses. Rust has no Wi-Fi for this chip, so the radio and the SDK stay in C: `runtime/` is a small C++ file that hands the core to Rust as plain functions, and the `esp8266` crate is the safe side of it (time, serial, pins, i2c, Wi-Fi, udp, flash). The app is all Rust.

## Setup

```
cargo install espup     # or a release binary
espup install -t esp32
```

## Use

An app is a `no_std` staticlib crate:

```rust
#![no_std]

struct Blink;

impl esp8266::App for Blink {
    fn setup() -> Self { Blink }
    fn tick(&mut self) {}
}

esp8266::app!(Blink);
```

```toml
[lib]
crate-type = ["staticlib"]

[dependencies]
esp8266 = { path = "path/to/rust-esp8266/esp8266" }
```

and in `platformio.ini`:

```ini
lib_deps = symlink://path/to/rust-esp8266/runtime
custom_rust_crate = rust          ; folder of the app crate
custom_rust_profile = release     ; optional
```

`cargo-esp8266` does the build: `cargo +esp` with `-Zbuild-std=core` and PlatformIO's xtensa gcc as linker, then the archive is written again with its objects named `*.c.o`. The Arduino linker script sends code of other objects to IRAM, which overflows; `*.c.o` goes to flash, and `--gc-sections` still drops what is unused. PlatformIO only runs python hooks, so `platformio/rust.py` is a few lines that call it. Without PlatformIO, `cargo esp8266 build --out lib.a` works on its own.

At `opt-level = 0` the esp8266 backend fails to build `core` ("Cannot scavenge register without an emergency spill slot"), so an app's dev profile needs `opt-level = 1` or more.

`examples/hello` blinks the led and says hello on serial.

```
cd examples/hello
pio run -t upload -t monitor
```
