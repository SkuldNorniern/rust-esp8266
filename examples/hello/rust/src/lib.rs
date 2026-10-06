//! rust called from an Arduino sketch, and calling back into it.

#![no_std]

unsafe extern "C" {
    /// the sketch prints these bytes.
    fn hello_write(data: *const u8, len: usize);
}

fn write(s: &str) {
    // SAFETY: the pointer and length come from one live `&str`.
    unsafe { hello_write(s.as_ptr(), s.len()) };
}

/// a sum over u64 and u32 mixed, with more arguments than fit in
/// registers, so the calling convention is checked end to end.
#[unsafe(no_mangle)]
pub extern "C" fn hello_sum(a: u64, b: u32, c: u64, d: u32, e: u64, f: u32) -> u64 {
    a + u64::from(b) + c + u64::from(d) + e + u64::from(f)
}

/// says hello through the sketch.
#[unsafe(no_mangle)]
pub extern "C" fn hello_greet() {
    write("hello from rust\n");
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    write("rust panicked\n");
    loop {}
}
