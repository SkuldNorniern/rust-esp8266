//! `cargo esp8266 build --out <lib.a>`: builds the staticlib crate in the
//! current folder for the esp8266 and writes an archive the Arduino
//! linker puts in flash.
//!
//! options:
//!   --manifest-path <Cargo.toml>   the crate, if not the current folder
//!   --profile <name>               default release
//!   --target-dir <dir>             default the crate's target/
//!   --toolchain <dir>              the xtensa-lx106 gcc bin folder,
//!                                  default `ESP8266_TOOLCHAIN` or
//!                                  `PlatformIO`'s `toolchain-xtensa`

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod archive;

const TARGET: &str = "xtensa-esp8266-none-elf";

struct Options {
    manifest: Option<PathBuf>,
    profile: String,
    target_dir: Option<PathBuf>,
    toolchain: Option<PathBuf>,
    out: PathBuf,
}

fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let mut manifest = None;
    let mut profile = String::from("release");
    let mut target_dir = None;
    let mut toolchain = None;
    let mut out = None;
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy().into_owned();
        let mut value = || {
            args.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--manifest-path" => manifest = Some(value()?),
            "--profile" => profile = value()?.to_string_lossy().into_owned(),
            "--target-dir" => target_dir = Some(value()?),
            "--toolchain" => toolchain = Some(value()?),
            "--out" => out = Some(value()?),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(Options {
        manifest,
        profile,
        target_dir,
        toolchain,
        out: out.ok_or("--out <lib.a> is needed")?,
    })
}

/// the folder holding `xtensa-lx106-elf-gcc`.
fn toolchain(given: Option<PathBuf>) -> Result<PathBuf, String> {
    let found = given
        .or_else(|| env::var_os("ESP8266_TOOLCHAIN").map(PathBuf::from))
        .or_else(|| {
            let home = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"))?;
            Some(Path::new(&home).join(".platformio/packages/toolchain-xtensa/bin"))
        })
        .ok_or("no toolchain: pass --toolchain or set ESP8266_TOOLCHAIN")?;
    if found.is_dir() {
        Ok(found)
    } else {
        Err(format!("no toolchain at {}", found.display()))
    }
}

fn exe(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}{}", env::consts::EXE_SUFFIX))
}

fn build(o: Options) -> Result<(), String> {
    let bin = toolchain(o.toolchain)?;
    let mut path = OsString::from(&bin);
    path.push(if cfg!(windows) { ";" } else { ":" });
    path.push(env::var_os("PATH").unwrap_or_default());

    let artifacts = o
        .out
        .parent()
        .unwrap_or(Path::new("."))
        .join("esp8266-artifacts");
    let _ = fs::remove_dir_all(&artifacts);

    let mut cargo = Command::new("cargo");
    cargo
        .args(["+esp", "build", "--target", TARGET, "-Zbuild-std=core"])
        .args(["-Zunstable-options", "--artifact-dir"])
        .arg(&artifacts)
        .args(["--profile", &o.profile])
        .env("PATH", path)
        // set by an outer cargo run, and wrong for the esp toolchain
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTC");
    if let Some(m) = &o.manifest {
        cargo.arg("--manifest-path").arg(m);
    }
    if let Some(t) = &o.target_dir {
        cargo.arg("--target-dir").arg(t);
    }
    let status = cargo.status().map_err(|e| format!("cargo: {e}"))?;
    if !status.success() {
        return Err(String::from("cargo build failed"));
    }

    let built = fs::read_dir(&artifacts)
        .map_err(|e| format!("{}: {e}", artifacts.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "a"))
        .ok_or("the crate built no staticlib: set crate-type = [\"staticlib\"]")?;
    let data = fs::read(&built).map_err(|e| format!("{}: {e}", built.display()))?;
    archive::for_flash(
        &data,
        &o.out,
        &exe(&bin, "xtensa-lx106-elf-ar"),
        &exe(&bin, "xtensa-lx106-elf-objcopy"),
    )?;
    let _ = fs::remove_dir_all(&artifacts);
    println!("cargo-esp8266: {}", o.out.display());
    Ok(())
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1).peekable();
    // `cargo esp8266 ...` passes its own name first
    if args.peek().is_some_and(|a| a == "esp8266") {
        args.next();
    }
    let result = match args.next().as_deref().and_then(|a| a.to_str()) {
        Some("build") => parse(args).and_then(build),
        _ => Err(String::from(
            "usage: cargo esp8266 build --out <lib.a> [options]",
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cargo-esp8266: {e}");
            ExitCode::FAILURE
        }
    }
}
