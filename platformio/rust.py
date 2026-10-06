# builds a rust crate for the esp8266 and links it into a PlatformIO
# project. in platformio.ini:
#
#   extra_scripts = pre:path/to/rust-esp8266/platformio/rust.py
#   custom_rust_crate = rust          ; the crate folder, a staticlib
#   custom_rust_profile = release     ; optional
#
# needs the esp toolchain (espup) for `cargo +esp`.

import json
import os
import shutil
import subprocess
import sys

Import("env")  # noqa: F821  (PlatformIO gives it)

TARGET = "xtensa-esp8266-none-elf"


def option(name, default=None):
    value = env.GetProjectOption(name, default)  # noqa: F821
    if value is None:
        sys.exit(f"rust-esp8266: set {name} in platformio.ini")
    return value


def members(path):
    """the objects in a gnu ar archive, as (name, bytes)."""
    with open(path, "rb") as f:
        data = f.read()
    if data[:8] != b"!<arch>\n":
        sys.exit(f"rust-esp8266: {path} is not an archive")
    at, names = 8, b""
    while at + 60 <= len(data):
        header = data[at : at + 60]
        name = header[:16].rstrip()
        size = int(header[48:58])
        body = data[at + 60 : at + 60 + size]
        at += 60 + size + (size & 1)
        if name == b"//":
            names = body
        elif name in (b"/", b"/SYM64/", b"__.SYMDEF"):
            continue
        else:
            if name.startswith(b"/"):
                start = int(name[1:])
                name = names[start : names.index(b"/\n", start)]
            yield name.rstrip(b"/").decode(), body


def flash_archive(source, out, ar):
    """the same objects, named `*.c.o`. the Arduino linker script sends
    `*.c.o(.text*)` to flash; any other object's code goes to IRAM and
    overflows it."""
    work = out + ".d"
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    files = []
    for i, (name, body) in enumerate(members(source)):
        if not name.endswith(".o"):
            continue
        path = os.path.join(work, f"rust_{i}.c.o")
        with open(path, "wb") as f:
            f.write(body)
        files.append(path)
    if os.path.exists(out):
        os.remove(out)
    subprocess.check_call([ar, "rcs", out] + files)
    shutil.rmtree(work)


def build():
    project = env.subst("$PROJECT_DIR")  # noqa: F821
    crate = os.path.join(project, option("custom_rust_crate"))
    profile = option("custom_rust_profile", "release")
    build_dir = env.subst("$BUILD_DIR")  # noqa: F821

    toolchain = env.PioPlatform().get_package_dir("toolchain-xtensa")  # noqa: F821
    bin_dir = os.path.join(toolchain, "bin")
    run_env = dict(os.environ)
    run_env["PATH"] = bin_dir + os.pathsep + run_env.get("PATH", "")
    target_dir = os.path.join(build_dir, "rust")

    command = [
        "cargo", "+esp", "build",
        "--target", TARGET,
        "-Zbuild-std=core",
        "--profile", profile,
        "--target-dir", target_dir,
    ]
    print("rust-esp8266:", " ".join(command))
    subprocess.check_call(command, cwd=crate, env=run_env)

    meta = subprocess.check_output(
        ["cargo", "+esp", "metadata", "--no-deps", "--format-version", "1"],
        cwd=crate, env=run_env,
    )
    package = json.loads(meta)["packages"][0]["name"].replace("-", "_")
    folder = "debug" if profile == "dev" else profile
    built = os.path.join(target_dir, TARGET, folder, f"lib{package}.a")

    out = os.path.join(build_dir, "librust_esp8266.a")
    ar = os.path.join(bin_dir, "xtensa-lx106-elf-ar" + (".exe" if os.name == "nt" else ""))
    flash_archive(built, out, ar)
    env.Append(LIBPATH=[build_dir], LIBS=["rust_esp8266"])  # noqa: F821


build()
