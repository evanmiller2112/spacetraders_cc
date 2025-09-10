// Global verbosity system for clean output control
use std::sync::atomic::{AtomicU8, Ordering};

static VERBOSITY_LEVEL: AtomicU8 = AtomicU8::new(0);

pub fn set_verbosity_level(level: u8) {
    VERBOSITY_LEVEL.store(level, Ordering::Relaxed);
    if level > 0 {
        println!("📢 Verbosity level: {} (0=quiet, 1=basic, 2=full)", level);
    }
}

pub fn get_verbosity_level() -> u8 {
    VERBOSITY_LEVEL.load(Ordering::Relaxed)
}

// Global verbosity macros that work anywhere
#[macro_export]
macro_rules! v_print {
    (0, $($arg:tt)*) => {
        if $crate::verbosity::get_verbosity_level() >= 0 {
            println!($($arg)*);
        }
    };
    (1, $($arg:tt)*) => {
        if $crate::verbosity::get_verbosity_level() >= 1 {
            println!($($arg)*);
        }
    };
    (2, $($arg:tt)*) => {
        if $crate::verbosity::get_verbosity_level() >= 2 {
            println!($($arg)*);
        }
    };
}

// Convenience macros
#[macro_export]
macro_rules! v_summary {
    ($($arg:tt)*) => { $crate::v_print!(0, $($arg)*); };
}

#[macro_export]
macro_rules! v_info {
    ($($arg:tt)*) => { $crate::v_print!(1, $($arg)*); };
}

#[macro_export]
macro_rules! v_debug {
    ($($arg:tt)*) => { $crate::v_print!(2, $($arg)*); };
}

// Trace level (level 2)
#[macro_export]
macro_rules! v_trace {
    ($($arg:tt)*) => { $crate::v_print!(2, $($arg)*); };
}

// Always print errors regardless of verbosity
#[macro_export]
macro_rules! v_error {
    ($($arg:tt)*) => { println!($($arg)*); };
}