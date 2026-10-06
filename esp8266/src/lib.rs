//! the Arduino esp8266 core from Rust. the radio and the SDK stay in C
//! (`runtime/esp8266.cpp`); this crate is the safe side of it. an app is a
//! type with `setup` and `tick`, handed to [`app!`].
//!
//! everything runs on the one Arduino loop: nothing here is called from
//! an interrupt, and nothing may be.

#![no_std]

use core::cell::RefCell;

mod sys;

pub mod flash;
pub mod i2c;
pub mod pin;
pub mod serial;
pub mod time;
pub mod udp;
pub mod wifi;

/// a firmware: made once at boot, then run round after round.
pub trait App {
    fn setup() -> Self;
    fn tick(&mut self);
}

/// holds the app between rounds. public for [`app!`] only.
#[doc(hidden)]
pub struct Slot<A>(pub RefCell<Option<A>>);

// SAFETY: the Arduino core runs setup and loop on one thread, and nothing
// reaches the slot from an interrupt.
unsafe impl<A> Sync for Slot<A> {}

impl<A: App> Slot<A> {
    #[must_use]
    pub const fn new() -> Self {
        Self(RefCell::new(None))
    }

    pub fn setup(&self) {
        *self.0.borrow_mut() = Some(A::setup());
    }

    pub fn tick(&self) {
        if let Some(app) = self.0.borrow_mut().as_mut() {
            app.tick();
        }
    }
}

impl<A: App> Default for Slot<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// makes `$app` the firmware: the Arduino core's `setup` and `loop`.
#[macro_export]
macro_rules! app {
    ($app:ty) => {
        static APP: $crate::Slot<$app> = $crate::Slot::new();

        #[unsafe(no_mangle)]
        pub extern "C" fn setup() {
            APP.setup();
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn r#loop() {
            APP.tick();
        }
    };
}

#[cfg(feature = "panic-handler")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    let _ = writeln!(serial::Serial, "panic: {info}");
    time::delay(100);
    time::restart()
}
