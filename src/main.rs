#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#[cfg(target_os = "windows")]
use runtime::run;
#[cfg(target_os = "windows")]
use runtime::show_error;

#[cfg(all(target_os = "windows", debug_assertions))]
macro_rules! debug_log {
    ($($argument:tt)*) => {{
        eprintln!("[app-activate] {}", format_args!($($argument)*))
    }};
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
macro_rules! debug_log {
    ($($argument:tt)*) => {{
        if false {
            let _ = format_args!($($argument)*);
        }
    }};
}

#[cfg(any(target_os = "windows", test))]
mod config;
#[cfg(any(target_os = "windows", test))]
mod decoder;
#[cfg(any(target_os = "windows", test))]
mod target;

#[cfg(target_os = "windows")]
mod activator;
#[cfg(target_os = "windows")]
mod runtime;

#[cfg(target_os = "windows")]
fn main() {
    if let Err(message) = run() {
        show_error(&message);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("app-activate can only run on Windows.");
}
