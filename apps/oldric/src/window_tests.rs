//! The whole window, run headless: the two apps sharing it, handing documents across, files
//! finding their app, and closing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::Harness;
use serde_json::json;

use crate::bar::Action;
use crate::route::App;
use crate::shell::Suite;

fn harness(files: &[String]) -> Harness<'static, Suite> {
    let files = files.to_vec();
    Harness::builder().with_size(egui::vec2(1280.0, 800.0)).with_max_steps(1_000_000).build_eframe(move |cc| Suite::plain(cc, &files))
}

/// A fresh scratch folder for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oldric-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn steps(h: &mut Harness<'_, Suite>, n: usize) {
    for _ in 0..n {
        h.step();
    }
}

/// Whether the last frame asked the window to close.
fn close_sent(h: &Harness<'_, Suite>) -> bool {
    h.output().viewport_output.values().any(|v| v.commands.iter().any(|c| matches!(c, egui::ViewportCommand::Close)))
}

/// Whether the last frame kept the window from closing.
fn close_cancelled(h: &Harness<'_, Suite>) -> bool {
    h.output().viewport_output.values().any(|v| v.commands.iter().any(|c| matches!(c, egui::ViewportCommand::CancelClose)))
}

/// Step until the window asks to close, up to `n` frames.
fn closes_within(h: &mut Harness<'_, Suite>, n: usize) -> bool {
    (0..n).any(|_| {
        h.step();
        close_sent(h)
    })
}

/// The window's close button.
fn click_close(h: &mut Harness<'_, Suite>) {
    h.input_mut().viewports.entry(egui::ViewportId::ROOT).or_default().events.push(egui::ViewportEvent::Close);
}

/// A report saved as `Report.docx` in `dir`, and the PDF exported from it.
fn report_files(dir: &Path) -> (String, String) {
    let mut h = harness(&[]);
    steps(&mut h, 3);
    let word = h.state_mut().word();
    word.run("file.new", json!({"template": "report"})).unwrap();
    let docx = dir.join("Report.docx").to_string_lossy().into_owned();
    let pdf = dir.join("Exported.pdf").to_string_lossy().into_owned();
    word.run("file.saveAs", json!({"path": docx})).unwrap();
    word.run("file.exportPdf", json!({"path": pdf})).unwrap();
    (docx, pdf)
}

fn word_path(h: &mut Harness<'_, Suite>) -> Option<String> {
    h.state_mut().word().session.path.as_ref().map(|p| p.to_string_lossy().into_owned())
}

fn pdf_paths(h: &mut Harness<'_, Suite>) -> Vec<String> {
    let pdf = h.state_mut().pdf();
    pdf.views.iter().filter_map(|v| pdf.session.get(v.id).and_then(|d| d.path.clone())).collect()
}

#[derive(Debug)]
struct DroppedPath(PathBuf);

impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

/// One frame, with the input through the suite's `raw_input_hook` first, as eframe does
/// (kittest calls only `logic` and `ui`).
fn step_hooked(h: &mut Harness<'_, Suite>) {
    let ctx = h.ctx.clone();
    let mut raw = std::mem::take(h.input_mut());
    eframe::App::raw_input_hook(h.state_mut(), &ctx, &mut raw);
    *h.input_mut() = raw;
    h.step();
}

fn drop_file(h: &mut Harness<'_, Suite>, path: &str) {
    h.input_mut().dropped_files.push(Arc::new(DroppedPath(PathBuf::from(path))));
    step_hooked(h);
}

#[test]
fn files_named_at_start_open_in_their_own_app() {
    let dir = scratch("start");
    let (docx, pdf) = report_files(&dir);
    let mut h = harness(&[docx.clone(), pdf.clone()]);
    steps(&mut h, 4);
    assert_eq!(word_path(&mut h), Some(docx.clone()));
    assert_eq!(pdf_paths(&mut h), vec![pdf.clone()]);
    // The first file named decides the app in view.
    assert_eq!(h.state().shown(), Some(App::Word));
    assert!(!h.state_mut().word().ui.backstage, "the document shows, not the Start screen");
    let mut h = harness(&[pdf, docx]);
    steps(&mut h, 4);
    assert_eq!(h.state().shown(), Some(App::Pdf));
    // Nothing named: WordCraft's Start screen.
    let mut h = harness(&[]);
    steps(&mut h, 4);
    assert_eq!(h.state().shown(), Some(App::Word));
    assert!(h.state_mut().word().ui.backstage);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_document_goes_to_the_pdf_tab_and_back_to_the_word_tab() {
    let dir = scratch("handoff");
    let mut h = harness(&[]);
    steps(&mut h, 3);
    let docx = dir.join("Report.docx");
    {
        let word = h.state_mut().word();
        word.run("file.new", json!({"template": "report"})).unwrap();
        word.run("file.saveAs", json!({"path": docx.to_string_lossy()})).unwrap();
    }
    // Open as PDF: written beside the document, shown in the PDF tab, kept in sync.
    h.state_mut().act(Action::OpenAsPdf);
    steps(&mut h, 4);
    let pdf = dir.join("Report.pdf").to_string_lossy().into_owned();
    assert_eq!(h.state().shown(), Some(App::Pdf));
    assert_eq!(pdf_paths(&mut h), vec![pdf.clone()]);
    assert!(h.state_mut().word().session.pdf_link.is_some());
    // Asking again shows the same tab rather than a second one.
    h.state_mut().act(Action::OpenAsPdf);
    steps(&mut h, 4);
    assert_eq!(pdf_paths(&mut h), vec![pdf]);
    // Edit in Word: a Word document beside the PDF, under a name no file has, in the Word tab.
    h.state_mut().act(Action::EditInWord);
    steps(&mut h, 4);
    assert_eq!(h.state().shown(), Some(App::Word));
    assert_eq!(word_path(&mut h), Some(dir.join("Report (2).docx").to_string_lossy().into_owned()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn live_sync_writes_waiting_edits_when_the_pdf_tab_opens() {
    let dir = scratch("sync");
    let mut h = harness(&[]);
    steps(&mut h, 3);
    let docx = dir.join("Report.docx");
    {
        let word = h.state_mut().word();
        word.run("file.new", json!({"template": "report"})).unwrap();
        word.ui.backstage = false;
        word.run("file.saveAs", json!({"path": docx.to_string_lossy()})).unwrap();
    }
    h.state_mut().act(Action::OpenAsPdf);
    steps(&mut h, 3);
    h.state_mut().show(App::Word);
    steps(&mut h, 3);
    let pdf = dir.join("Report.pdf");
    std::fs::write(&pdf, b"stale").unwrap();
    h.state_mut().word().run("text.insert", json!({"text": "Typed "})).unwrap();
    steps(&mut h, 2);
    // Still within the pause after typing: not written yet.
    assert_eq!(std::fs::read(&pdf).unwrap(), b"stale");
    h.state_mut().show(App::Pdf);
    steps(&mut h, 2);
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"), "written on the way to the PDF tab");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dropped_files_go_to_the_app_that_edits_them() {
    let dir = scratch("drop");
    let (docx, pdf) = report_files(&dir);
    let mut h = harness(&[]);
    steps(&mut h, 3);
    // A PDF dropped on WordCraft opens in PdfCraft.
    drop_file(&mut h, &pdf);
    steps(&mut h, 4);
    assert_eq!(h.state().shown(), Some(App::Pdf));
    assert_eq!(pdf_paths(&mut h), vec![pdf]);
    // A Word document dropped on PdfCraft opens in WordCraft.
    drop_file(&mut h, &docx);
    steps(&mut h, 4);
    assert_eq!(h.state().shown(), Some(App::Word));
    assert_eq!(word_path(&mut h), Some(docx));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ctrl_alt_page_keys_switch_apps_and_reach_neither() {
    let mut h = harness(&[]);
    steps(&mut h, 3);
    let press = |h: &mut Harness<'_, Suite>, key| {
        h.input_mut().events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND | egui::Modifiers::ALT,
        });
        step_hooked(h);
    };
    press(&mut h, egui::Key::PageDown);
    assert_eq!(h.state().shown(), Some(App::Pdf));
    press(&mut h, egui::Key::PageUp);
    assert_eq!(h.state().shown(), Some(App::Word));
    // Back and forth, each app draws with its own fonts and theme again (named font families
    // that the other app doesn't install would panic while laying out).
    for _ in 0..3 {
        press(&mut h, egui::Key::PageDown);
        steps(&mut h, 3);
        press(&mut h, egui::Key::PageDown);
        steps(&mut h, 3);
    }
    assert_eq!(h.state().shown(), Some(App::Word));
}

#[test]
fn each_app_draws_with_its_own_look_after_a_switch() {
    let mut h = harness(&[]);
    steps(&mut h, 4);
    let look = |h: &Harness<'_, Suite>| {
        let style = h.ctx.global_style();
        (style.visuals.panel_fill, style.visuals.window_fill, style.spacing.button_padding)
    };
    let word = look(&h);
    h.state_mut().show(App::Pdf);
    steps(&mut h, 4);
    let pdf = look(&h);
    assert_ne!(word, pdf, "the two apps style egui differently");
    h.state_mut().show(App::Word);
    steps(&mut h, 4);
    assert_eq!(look(&h), word, "WordCraft installs its look again");
    h.state_mut().show(App::Pdf);
    steps(&mut h, 4);
    assert_eq!(look(&h), pdf, "and so does PdfCraft");
}

#[test]
fn closing_asks_each_app_about_unsaved_work_in_turn() {
    let mut h = harness(&[]);
    steps(&mut h, 3);
    // Unsaved typing in WordCraft and an unsaved PDF.
    {
        let word = h.state_mut().word();
        word.ui.backstage = false;
        word.run("text.insert", json!({"text": "Unsaved"})).unwrap();
    }
    let dir = scratch("close");
    let (_, pdf) = report_files(&dir);
    h.state_mut().pdf().open_path(&pdf);
    h.state_mut().show(App::Pdf);
    steps(&mut h, 4);
    h.state_mut().pdf().run_command("page.insert_blank");
    steps(&mut h, 2);
    assert!(h.state_mut().pdf().first_dirty().is_some());

    // PdfCraft is in view: it asks first.
    click_close(&mut h);
    assert!(!closes_within(&mut h, 3));
    assert_eq!(h.state().shown(), Some(App::Pdf));
    assert!(matches!(h.state_mut().pdf().close_request, Some(pdfcraft_ui_egui::CloseRequest::Quit)));
    // Cancel keeps everything.
    h.state_mut().pdf().close_request = None;
    steps(&mut h, 2);

    // From the Word tab, WordCraft asks first; its answer lets the close go on to PdfCraft.
    h.state_mut().show(App::Word);
    steps(&mut h, 3);
    click_close(&mut h);
    assert!(!closes_within(&mut h, 3));
    assert_eq!(h.state_mut().word().dialog.as_ref().map(|d| d.name()), Some("saveChanges"));
    h.state_mut().word().run("ui.saveChanges", json!({"answer": "dontSave"})).unwrap();
    // The close is sent again, and PdfCraft (still unsaved) asks in turn.
    assert!(closes_within(&mut h, 3), "the window's close goes on after WordCraft's answer");
    click_close(&mut h);
    assert!(!closes_within(&mut h, 3));
    assert_eq!(h.state().shown(), Some(App::Pdf));
    assert!(matches!(h.state_mut().pdf().close_request, Some(pdfcraft_ui_egui::CloseRequest::Quit)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn wordcrafts_close_closes_the_document_not_the_window() {
    let mut h = harness(&[]);
    steps(&mut h, 3);
    {
        let word = h.state_mut().word();
        word.ui.backstage = false;
        word.run("file.new", json!({"template": "report"})).unwrap();
        word.session.dirty = false;
        word.run("file.close", json!({})).unwrap();
    }
    assert!(!closes_within(&mut h, 4));
    let word = h.state_mut().word();
    assert!(!word.quit_requested);
    assert!(word.session.path.is_none());
    let text = word.run("document.text", json!({})).unwrap();
    assert_eq!(text["text"].as_str().map(str::trim), Some(""), "a blank document is left");
    assert!(word.ui.backstage, "behind the Start screen");
    // Nothing unsaved: the window's close goes ahead.
    click_close(&mut h);
    h.step();
    assert!(!close_cancelled(&h));
}
