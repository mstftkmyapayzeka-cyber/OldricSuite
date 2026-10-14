//! Oldric Suite: WordCraft (documents) and PdfCraft (PDFs) in one window. The `oldric` program
//! (`main.rs`) opens the window; [`shell::Suite`] is what it shows.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod bar;
pub mod icon;
pub mod route;
pub mod shell;
pub mod text;

#[cfg(test)]
mod window_tests;
