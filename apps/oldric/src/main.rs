//! Oldric Suite: WordCraft (documents) and PdfCraft (PDFs) in one window.
//!
//! Usage: `oldric [files…]`. Word, OpenDocument, RTF, text and web documents open in the Word tab;
//! PDFs and pictures in the PDF tab. `--version` prints the version.
//!
//! Settings: WordCraft's are its own (shared with the `wordcraft` program); PdfCraft's are kept in
//! the suite's settings folder (`app.ron`, copied from PdfCraft's on the first run), with the log
//! files in `logs`. `OLDRIC_RENDERER=gl` draws with OpenGL instead of wgpu.

// Release builds on Windows are GUI-subsystem programs: no console window beside the app.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use oldric::{icon, shell};

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

/// The app id: the Wayland app id and the settings folder's name.
const APP_ID: &str = "oldric-suite";

/// The suite's settings folder (PdfCraft's preferences, the logs).
fn settings_dir() -> Option<PathBuf> {
    eframe::storage_dir("Oldric Suite")
}

/// PdfCraft's preferences come along the first time the suite runs: its `app.ron`, copied once.
fn adopt_pdfcraft_settings(dir: &std::path::Path) {
    let ours = dir.join("app.ron");
    if ours.exists() {
        return;
    }
    let Some(theirs) = pdfcraft::settings_dir().map(|d| d.join("app.ron")).filter(|p| p.is_file()) else { return };
    if let Err(e) = std::fs::create_dir_all(dir).and_then(|()| std::fs::copy(&theirs, &ours)) {
        log::warn!("copying PdfCraft's settings from {}: {e}", theirs.display());
    }
}

fn native_options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    // PdfCraft's window set-up, GPU choice included; the suite's own name, icon and settings file.
    let mut native = pdfcraft::native_options(false, false, renderer);
    native.viewport = native
        .viewport
        .with_title("Oldric")
        .with_app_id(APP_ID)
        .with_icon(icon::icon())
        .with_inner_size([1440.0, 940.0])
        .with_min_inner_size([860.0, 540.0]);
    native.persistence_path = settings_dir().map(|d| d.join("app.ron"));
    #[cfg(all(unix, not(target_os = "macos")))]
    // winit has no file drag-and-drop on Wayland (only on X11): run through XWayland when it's
    // there, as WordCraft does. OLDRIC_WAYLAND=1 keeps the native Wayland backend.
    if std::env::var_os("DISPLAY").is_some() && std::env::var_os("OLDRIC_WAYLAND").is_none() {
        native.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            b.with_x11();
        }));
    }
    native
}

fn main() -> eframe::Result {
    let logger = pdfcraft::logging::install();
    std::panic::set_hook(Box::new(|info| {
        let trace = std::backtrace::Backtrace::capture();
        let report = if trace.status() == std::backtrace::BacktraceStatus::Captured {
            format!("internal error: {info}\n{trace}")
        } else {
            format!("internal error: {info}")
        };
        if log::log_enabled!(log::Level::Error) {
            log::error!("{report}");
        } else {
            // `eprintln!` panics on a broken stderr pipe, and a panic inside the panic hook aborts.
            let _ = std::io::Write::write_fmt(&mut std::io::stderr(), format_args!("oldric: {report}\n"));
        }
    }));
    let mut files = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" => {
                println!("oldric {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            flag if flag.starts_with("--") => log::warn!("unknown option {flag}"),
            _ => files.push(pdfcraft::path_from_arg(arg)),
        }
    }
    let files = pdfcraft::rejoin_split_paths(files, |p| std::path::Path::new(p).is_file());
    if let Some(dir) = settings_dir() {
        if let Some(logger) = logger {
            match logger.attach_dir(&dir.join("logs")) {
                Ok(path) => log::info!("Oldric Suite {}, log file {}", env!("CARGO_PKG_VERSION"), path.display()),
                Err(e) => log::warn!("no log file: {e}"),
            }
        }
        adopt_pdfcraft_settings(&dir);
    }
    #[cfg(target_os = "macos")]
    let apple_events = pdfcraft::apple_events::AppleEvents::install();
    let choice = pdfcraft::renderer_choice(std::env::var("OLDRIC_RENDERER").ok().as_deref());
    let first = if choice == pdfcraft::RendererChoice::Gl { eframe::Renderer::Glow } else { eframe::Renderer::Wgpu };
    let started = Rc::new(Cell::new(false));
    let result = eframe::run_native(
        "Oldric",
        native_options(first),
        creator(
            Rc::clone(&started),
            files.clone(),
            #[cfg(target_os = "macos")]
            &apple_events,
        ),
    );
    match result {
        // wgpu couldn't start on this GPU or driver: OpenGL usually still works (PdfCraft #461).
        Err(e @ eframe::Error::Wgpu(_)) if pdfcraft::retry_with_gl(choice, started.get()) => {
            log::error!("the GPU renderer (wgpu) didn't start: {e}. Starting with OpenGL instead; set OLDRIC_RENDERER=gl to skip wgpu.");
            eframe::run_native(
                "Oldric",
                native_options(eframe::Renderer::Glow),
                creator(
                    started,
                    files,
                    #[cfg(target_os = "macos")]
                    &apple_events,
                ),
            )
        }
        other => other,
    }
}

/// The suite, once the window and renderer are up (`started` says so, for the OpenGL retry).
fn creator<'a>(
    started: Rc<Cell<bool>>,
    files: Vec<String>,
    #[cfg(target_os = "macos")] apple_events: &'a pdfcraft::apple_events::AppleEvents,
) -> eframe::AppCreator<'a> {
    Box::new(move |cc| {
        started.set(true);
        let suite = shell::Suite::new(cc, &files);
        #[cfg(target_os = "macos")]
        let suite = suite.with_os_events(apple_events.connect(&cc.egui_ctx));
        Ok(Box::new(suite))
    })
}
