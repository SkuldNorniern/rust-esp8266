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


# the loop runs on the core's 4 KiB stack and nothing grows it: rustc
# inlining a few buffers into one function can overflow it at run time
# ("Stack smashing detected"). after linking, every rust function's frame
# is read from the code and a frame past the limit fails the build.
def frames(target, source, env):
    import re

    objdump = os.path.join(
        firmware.PioPlatform().get_package_dir("toolchain-xtensa"), "bin", "xtensa-lx106-elf-objdump"
    )
    limit = int(firmware.GetProjectOption("custom_rust_frame_limit", "1536"))
    text = subprocess.run(
        [objdump, "-d", str(target[0])], capture_output=True, text=True, check=True
    ).stdout
    sizes = {}
    name, line_no = None, 0
    for line in text.splitlines():
        head = re.match(r"^[0-9a-f]+ <(.*)>:$", line)
        if head:
            name, line_no = head.group(1), 0
            continue
        if name is None:
            continue
        line_no += 1
        if line_no > 6:
            continue
        # a big frame: movi a8, -N; add.n a8, a1, a8; mov.n a1, a8
        big = re.search(r"movi\s+a8, (0x[0-9a-f]+|-?\d+)$", line)
        if big:
            value = int(big.group(1), 0)
            if value >= 1 << 31:
                value -= 1 << 32
            if value < 0:
                sizes[name] = -value
        small = re.search(r"addi\s+a1, a1, (-\d+)", line)
        if small:
            sizes[name] = sizes.get(name, 0) - int(small.group(1))
    rust = sorted(
        ((size, n) for n, size in sizes.items() if n.startswith("_R") or n == "loop"),
        reverse=True,
    )
    for size, n in rust[:3]:
        print(f"rust frame {size} B  {n[:100]}")
    over = [(size, n) for size, n in rust if size > limit]
    if over:
        for size, n in over:
            print(f"error: rust frame {size} B over the {limit} B limit: {n}")
        return 1
    return 0


firmware.AddPostAction("$BUILD_DIR/${PROGNAME}.elf", frames)
