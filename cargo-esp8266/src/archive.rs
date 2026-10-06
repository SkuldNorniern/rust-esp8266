//! the Arduino esp8266 linker script sends `*.c.o(.text*)` to flash and
//! code of any other object to IRAM, which then overflows. so the objects
//! of a rust archive go out again named `*.c.o`; `--gc-sections` still
//! drops what is unused.
//!
//! `compiler_builtins` defines the arithmetic helpers the chip's ROM has
//! too (`__udivsi3` and friends). linked, its flash copies would win over
//! the ROM's for everyone, the SDK's interrupt code included, which runs
//! while flash is off and dies on them (illegal instruction at boot). so
//! those are kept local to the rust objects, and every other caller gets
//! the ROM's.

use std::fs;
use std::path::Path;
use std::process::Command;

const MAGIC: &[u8] = b"!<arch>\n";
/// the helpers `eagle.rom.addr.v6.ld` gives from ROM.
const ROM: [&str; 24] = [
    "__adddf3",
    "__addsf3",
    "__divdf3",
    "__divdi3",
    "__divsi3",
    "__extendsfdf2",
    "__fixdfsi",
    "__fixunsdfsi",
    "__fixunssfsi",
    "__floatsidf",
    "__floatsisf",
    "__floatunsidf",
    "__floatunsisf",
    "__muldf3",
    "__muldi3",
    "__mulsf3",
    "__subdf3",
    "__subsf3",
    "__truncdfsf2",
    "__udivdi3",
    "__udivsi3",
    "__umoddi3",
    "__umodsi3",
    "__umulsidi3",
];
const HEADER: usize = 60;

/// the objects in a gnu ar archive, symbol tables left out.
fn objects(data: &[u8]) -> Result<Vec<&[u8]>, String> {
    let body = data.strip_prefix(MAGIC).ok_or("not an ar archive")?;
    let mut out = Vec::new();
    let mut at = 0;
    while at + HEADER <= body.len() {
        let header = &body[at..at + HEADER];
        let name = std::str::from_utf8(&header[..16]).unwrap_or("").trim_end();
        let size: usize = std::str::from_utf8(&header[48..58])
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .ok_or("broken member header")?;
        let start = at + HEADER;
        let member = body.get(start..start + size).ok_or("member cut short")?;
        at = start + size + (size & 1);
        // "/" and "/SYM64/" are symbol tables, "//" the long names
        if name.starts_with('/') && !name[1..].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        if member.starts_with(b"\x7fELF") {
            out.push(member);
        }
    }
    Ok(out)
}

/// writes the objects of `data` to `out` as `rust_<n>.c.o`, indexed by
/// `ar` so the linker finds their symbols, with the ROM's helpers made
/// local by `objcopy`.
pub fn for_flash(data: &[u8], out: &Path, ar: &Path, objcopy: &Path) -> Result<(), String> {
    let objects = objects(data)?;
    let work = out.with_extension("objects");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let mut names = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        let path = work.join(format!("rust_{i}.c.o"));
        fs::write(&path, object).map_err(|e| format!("{}: {e}", path.display()))?;
        names.push(path);
    }
    for path in &names {
        let status = Command::new(objcopy)
            .args(ROM.map(|s| format!("--localize-symbol={s}")))
            .arg(path)
            .status()
            .map_err(|e| format!("{}: {e}", objcopy.display()))?;
        if !status.success() {
            let _ = fs::remove_dir_all(&work);
            return Err(format!("objcopy failed on {}", path.display()));
        }
    }
    let _ = fs::remove_file(out);
    let status = Command::new(ar)
        .arg("rcs")
        .arg(out)
        .args(&names)
        .status()
        .map_err(|e| format!("{}: {e}", ar.display()))?;
    let _ = fs::remove_dir_all(&work);
    if status.success() {
        Ok(())
    } else {
        Err(String::from("ar failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, body: &[u8]) -> Vec<u8> {
        let mut m = format!(
            "{name:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
            0,
            0,
            0,
            644,
            body.len()
        )
        .into_bytes();
        m.extend_from_slice(body);
        if body.len() % 2 == 1 {
            m.push(b'\n');
        }
        m
    }

    #[test]
    fn objects_come_out_and_symbol_tables_and_names_stay_behind() {
        let mut a = MAGIC.to_vec();
        a.extend(member("/", b"symbols"));
        a.extend(member("//", b"a-long-object-name.o/\n"));
        a.extend(member("/0", b"\x7fELFone"));
        a.extend(member("short.o/", b"\x7fELFtwo!"));
        a.extend(member("lib.rmeta/", b"not code"));
        let got = objects(&a).unwrap();
        assert_eq!(got, [&b"\x7fELFone"[..], &b"\x7fELFtwo!"[..]]);
        assert!(objects(b"nope").is_err());
    }
}
