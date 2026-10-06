# PlatformIO only runs python hooks; the work is cargo-esp8266. the
# runtime library runs this; a project only needs, in platformio.ini:
#
#   lib_deps = symlink://path/to/rust-esp8266/esp8266-sys/runtime
#   custom_rust_crate = rust          ; folder of a staticlib crate
#   custom_rust_profile = release     ; optional

import inspect
import os
import subprocess

Import("env")  # noqa: F821

# the library's env builds the runtime; the firmware links in the global one
firmware = DefaultEnvironment()  # noqa: F821
# SCons runs this without __file__
here = os.path.dirname(os.path.abspath(inspect.getframeinfo(inspect.currentframe()).filename))
build = firmware.subst("$BUILD_DIR")
crate = os.path.join(firmware.subst("$PROJECT_DIR"), firmware.GetProjectOption("custom_rust_crate"))
subprocess.check_call([
    "cargo", "run", "-q", "--release",
    "--manifest-path", os.path.join(here, "..", "..", "Cargo.toml"), "-p", "cargo-esp8266",
    "--", "build",
    "--manifest-path", os.path.join(crate, "Cargo.toml"),
    "--profile", firmware.GetProjectOption("custom_rust_profile", "release"),
    "--target-dir", os.path.join(build, "rust"),
    "--toolchain", os.path.join(firmware.PioPlatform().get_package_dir("toolchain-xtensa"), "bin"),
    "--out", os.path.join(build, "librust_esp8266.a"),
])
firmware.Append(LIBPATH=[build], LIBS=["rust_esp8266"])
