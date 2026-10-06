# rust-esp8266

Rust on the ESP8266, linked into Arduino firmware with PlatformIO.

The esp Rust toolchain still has the `xtensa-esp8266-none-elf` target and builds call0 code, the same convention lx106 gcc uses. Rust has no Wi-Fi for this chip, so the radio and the SDK stay in C, and Rust is a `no_std` staticlib the sketch calls. This repo makes that one line in `platformio.ini`.

## Setup

```
cargo install espup     # or a release binary
espup install -t esp32
```

## Use

```ini
extra_scripts = pre:path/to/rust-esp8266/platformio/rust.py
custom_rust_crate = rust          ; folder of a staticlib crate
custom_rust_profile = release     ; optional
```

The script builds the crate with `cargo +esp` (`-Zbuild-std=core`), using PlatformIO's xtensa toolchain as linker, and links it into the firmware. The Arduino linker script sends code of unknown objects to IRAM, which then overflows, so the objects are renamed to `*.c.o` to land in flash; `--gc-sections` still drops what is unused.

`examples/hello` calls Rust from a sketch and back, with 64-bit and stack arguments.

```
cd examples/hello
pio run -t upload -t monitor
```
