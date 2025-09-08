// Debug logging system for comprehensive function call tracking
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Global flag for full debug mode
static FULL_DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Set the global full debug mode state
pub fn set_full_debug(enabled: bool) {
    FULL_DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Check if full debug mode is enabled
pub fn is_full_debug_enabled() -> bool {
    FULL_DEBUG_ENABLED.load(Ordering::Relaxed)
}

/// Get current timestamp for logging
pub fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Macro for logging function entry
#[macro_export]
macro_rules! debug_fn_enter {
    ($fn_name:expr) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🟢 [{}] ENTER: {}", timestamp, $fn_name);
        }
    };
    ($fn_name:expr, $($arg:tt)*) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🟢 [{}] ENTER: {} - {}", timestamp, $fn_name, format!($($arg)*));
        }
    };
}

/// Macro for logging function exit
#[macro_export]
macro_rules! debug_fn_exit {
    ($fn_name:expr) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🔴 [{}] EXIT:  {}", timestamp, $fn_name);
        }
    };
    ($fn_name:expr, $result:expr) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🔴 [{}] EXIT:  {} -> {:?}", timestamp, $fn_name, $result);
        }
    };
}

/// Macro for logging function execution with automatic entry/exit
#[macro_export]
macro_rules! debug_fn {
    ($fn_name:expr, $block:block) => {{
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🟢 [{}] ENTER: {}", timestamp, $fn_name);
            let result = $block;
            let timestamp = crate::debug::get_timestamp();
            println!("🔴 [{}] EXIT:  {} -> {:?}", timestamp, $fn_name, &result);
            result
        } else {
            $block
        }
    }};
}

/// Macro for logging API calls specifically
#[macro_export]
macro_rules! debug_api_call {
    ($method:expr, $url:expr) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🌐 [{}] API: {} {}", timestamp, $method, $url);
        }
    };
    ($method:expr, $url:expr, $body:expr) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("🌐 [{}] API: {} {} - Body: {:?}", timestamp, $method, $url, $body);
        }
    };
}

/// Macro for logging important debug information
#[macro_export]
macro_rules! debug_info {
    ($($arg:tt)*) => {
        if crate::debug::is_full_debug_enabled() {
            let timestamp = crate::debug::get_timestamp();
            println!("ℹ️  [{}] DEBUG: {}", timestamp, format!($($arg)*));
        }
    };
}

