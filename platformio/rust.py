# PlatformIO only runs python hooks; the work is cargo-esp8266. in
# platformio.ini:
#
#   extra_scripts = pre:path/to/rust-esp8266/platformio/rust.py
#   custom_rust_crate = rust          ; folder of a staticlib crate
#   custom_rust_profile = release     ; optional

import inspect
import os
import subprocess

Import("env")  # noqa: F821

# SCons runs this without __file__
here = os.path.dirname(os.path.abspath(inspect.getframeinfo(inspect.currentframe()).filename))
build = env.subst("$BUILD_DIR")  # noqa: F821
crate = os.path.join(env.subst("$PROJECT_DIR"), env.GetProjectOption("custom_rust_crate"))  # noqa: F821
subprocess.check_call([
    "cargo", "run", "-q", "--release",
    "--manifest-path", os.path.join(here, "..", "cargo-esp8266", "Cargo.toml"),
    "--", "build",
    "--manifest-path", os.path.join(crate, "Cargo.toml"),
    "--profile", env.GetProjectOption("custom_rust_profile", "release"),  # noqa: F821
    "--target-dir", os.path.join(build, "rust"),
    "--toolchain", os.path.join(env.PioPlatform().get_package_dir("toolchain-xtensa"), "bin"),  # noqa: F821
    "--out", os.path.join(build, "librust_esp8266.a"),
])
env.Append(LIBPATH=[build], LIBS=["rust_esp8266"])  # noqa: F821
