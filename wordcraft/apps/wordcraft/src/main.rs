//! The `wordcraft` program; the app itself is the library (`lib.rs`).
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    wordcraft::run()
}
