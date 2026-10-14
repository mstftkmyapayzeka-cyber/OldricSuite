//! The window: WordCraft and PdfCraft take turns drawing it under the suite bar.
//!
//! Only the app in view runs (its `logic` and `ui`); the other waits, its document and state
//! kept. Both apps set up egui to their own liking (fonts, theme, interface size, window title),
//! so the one coming into view installs its own again first
//! ([`wordcraft_ui_egui::WordApp::reclaim_context`], [`PdfCraftApp::reclaim_context`]) and skips
//! drawing until its fonts are in.
//!
//! The apps hand documents to each other inside the window instead of starting the other app:
//! WordCraft's Open in PdfCraft shows the PDF in the PDF tab, and Keep PDF in Sync rewrites it
//! as the document changes (PdfCraft reloads a file that changes on disk; the switch to the PDF
//! tab writes any edit still waiting, [`wordcraft_ui_egui::WordApp::flush_pdf_link`]). PdfCraft's
//! Edit in WordCraft opens the Word document it writes in the Word tab.

use std::sync::mpsc::{Receiver, Sender, channel};

use pdfcraft_ui_egui::{CloseRequest, PdfCraftApp};
use serde_json::json;

use crate::bar;
use crate::route::{self, App};

/// What one app asked of the other.
#[derive(Debug, PartialEq, Eq)]
pub enum Handoff {
    /// WordCraft wrote this PDF: show it in PdfCraft.
    ShowPdf(String),
    /// PdfCraft wrote this Word document: open it in WordCraft.
    EditWord(String),
}

pub struct Suite {
    word: wordcraft::Embedded,
    pdf: PdfCraftApp,
    /// The app to show.
    want: App,
    /// The app whose `logic` ran this frame and whose `ui` draws it.
    shown: Option<App>,
    handoffs: Receiver<Handoff>,
    /// Files to open in the app they belong to (dropped on the window, or from the system).
    pending: Vec<String>,
    /// The window's close is waiting on WordCraft's "save changes?" question.
    closing: bool,
    /// The title last sent for the window, and the app's own title it was made from.
    title: String,
    app_title: String,
    /// macOS: documents opened from Finder and the Dock's Quit.
    #[cfg(target_os = "macos")]
    os_events: Option<pdfcraft_ui_egui::OsEventsFn>,
}

impl Suite {
    /// Both apps, wired to each other and to this computer, with `files` open (each in its own
    /// app). Nothing to open starts on WordCraft's Start screen (File › Home).
    pub fn new(cc: &eframe::CreationContext<'_>, files: &[String]) -> Self {
        Self::build(cc, files, true)
    }

    /// [`Suite::new`] without PdfCraft's set-up for this computer (its saved preferences,
    /// printers, crash recovery and update checks): for tests.
    pub fn plain(cc: &eframe::CreationContext<'_>, files: &[String]) -> Self {
        Self::build(cc, files, false)
    }

    fn build(cc: &eframe::CreationContext<'_>, files: &[String], system: bool) -> Self {
        let (tx, handoffs) = channel();
        let word = wordcraft::Embedded::new(&cc.egui_ctx, word_hook(tx.clone()));
        let mut pdf = PdfCraftApp::new();
        if system {
            pdfcraft::configure_app(&mut pdf, cc);
        }
        pdf.word_host = Some(pdf_hook(tx));
        let mut suite = Suite {
            word,
            pdf,
            want: App::Word,
            shown: None,
            handoffs,
            pending: Vec::new(),
            closing: false,
            title: String::new(),
            app_title: String::new(),
            #[cfg(target_os = "macos")]
            os_events: None,
        };
        suite.open_at_start(files);
        suite
    }

    /// The WordCraft in the window.
    pub fn word(&mut self) -> &mut wordcraft_ui_egui::WordApp {
        &mut self.word.app
    }

    /// The PdfCraft in the window.
    pub fn pdf(&mut self) -> &mut PdfCraftApp {
        &mut self.pdf
    }

    /// The app drawing the window (from the first frame on).
    pub fn shown(&self) -> Option<App> {
        self.shown
    }

    /// Show `app` from the next frame.
    pub fn show(&mut self, app: App) {
        self.want = app;
    }

    #[cfg(target_os = "macos")]
    pub fn with_os_events(mut self, events: pdfcraft_ui_egui::OsEventsFn) -> Self {
        self.os_events = Some(events);
        self
    }

    fn open_at_start(&mut self, files: &[String]) {
        let pdfs: Vec<String> = files.iter().filter(|f| route::opener(f) == App::Pdf).cloned().collect();
        // With the preference on, last session's PDFs come back first (as in PdfCraft).
        self.pdf.reopen_last_files(&pdfs);
        let mut word_opened = false;
        for f in files {
            match route::opener(f) {
                App::Pdf => self.pdf.open_path(f),
                App::Word => {
                    word_opened = true;
                    self.open_word(f);
                }
            }
        }
        // The first file named decides which app shows; nothing named starts on WordCraft's Start
        // screen.
        self.want = files.first().map_or(App::Word, |f| route::opener(f));
        if !word_opened {
            self.word.app.start_screen();
        }
    }

    /// Open `path` in its app and show that app.
    fn open(&mut self, path: &str) {
        match route::opener(path) {
            App::Pdf => {
                self.pdf.show_path(path);
                self.want = App::Pdf;
            }
            App::Word => {
                self.open_word(path);
                self.want = App::Word;
            }
        }
    }

    /// Open `path` in WordCraft like File › Open: a document with unsaved changes asks first,
    /// and the open follows the answer. The Start screen (Backstage) gives way to the document.
    fn open_word(&mut self, path: &str) {
        let app = &mut self.word.app;
        match app.run("file.open", json!({"path": path})) {
            Ok(_) => app.ui.backstage = false,
            Err(e) => {
                log::warn!("{path}: {e}");
                let msg = wordcraft_ui_egui::tl!("Couldn't open {path}: {error}");
                app.status(wordcraft_ui_egui::i18n::fmt(msg, &[("path", path), ("error", &e.to_string())]));
            }
        }
    }

    fn take_handoffs(&mut self) {
        while let Ok(handoff) = self.handoffs.try_recv() {
            match handoff {
                Handoff::ShowPdf(path) => {
                    self.pdf.show_path(&path);
                    self.want = App::Pdf;
                }
                Handoff::EditWord(path) => {
                    self.open_word(&path);
                    self.want = App::Word;
                }
            }
        }
    }

    /// Bring `want` into view: the app leaving hands over what the other needs, and the one
    /// coming installs its look again.
    fn switch(&mut self, ctx: &egui::Context) {
        if self.shown == Some(self.want) {
            return;
        }
        if self.shown == Some(App::Word) {
            // PdfCraft should show every edit, including one typed just before the switch.
            self.word.app.flush_pdf_link();
        }
        match self.want {
            App::Word => {
                self.word.app.reclaim_context();
                // PdfCraft sets the mouse wheel's line speed (Preferences ▸ Scrolling); WordCraft
                // scrolls at egui's.
                let line = egui::InputOptions::default().line_scroll_speed;
                ctx.options_mut(|o| o.input_options.line_scroll_speed = line);
            }
            App::Pdf => self.pdf.reclaim_context(),
        }
        self.shown = Some(self.want);
        self.title.clear();
        ctx.request_repaint();
    }

    /// The window's close button (or the system) asked to quit: each app with unsaved work asks
    /// about it in turn, the one in view first. Returns whether the window may close now.
    fn may_close(&mut self, ctx: &egui::Context) -> bool {
        let word_asks = self.word.app.session.dirty && !self.word.app.quit_requested;
        let word_busy = self.word.app.printing.busy();
        let pdf_asks = self.pdf.quit_needs_answer();
        let word_first = word_busy || (word_asks && (self.shown == Some(App::Word) || !pdf_asks));
        if word_first {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            // Shows "save changes?" (or says a print job is still being sent).
            let _ = self.word.app.close_requested();
            self.closing = !word_busy;
            self.want = App::Word;
            return false;
        }
        if pdf_asks {
            if self.shown != Some(App::Pdf) {
                // PdfCraft asks when it sees the close itself; it isn't running this frame.
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pdf.close_request = Some(CloseRequest::Quit);
                self.want = App::Pdf;
            }
            // In view, PdfCraft cancels the close and asks in its own `logic`, this frame.
            return false;
        }
        // A clean quit: PdfCraft leaves nothing to recover (in view, its `logic` does this too).
        self.pdf.shutdown_recovery();
        true
    }

    /// WordCraft's Close (File › Close, Ctrl+W) closed its window in the `wordcraft` program; here
    /// it closes the document, leaving a blank one behind the Start screen. During the window's
    /// close it is WordCraft's answer to "save changes?", and the close goes on.
    fn word_closed(&mut self, ctx: &egui::Context) {
        if !self.word.app.quit_requested {
            // "Save changes?" was answered Cancel: the window stays.
            if self.closing && self.word.app.dialog.is_none() {
                self.closing = false;
            }
            return;
        }
        if self.closing {
            self.closing = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.word.app.quit_requested = false;
        // The answer to "save changes?" was given; nothing is asked again.
        self.word.app.session.dirty = false;
        if let Err(e) = self.word.app.run("file.new", json!({})) {
            log::warn!("closing the document: {e}");
        }
        self.word.app.start_screen();
    }

    /// What the window's title should be: the app's own, with the suite's name for the app's.
    fn sync_title(&mut self, ctx: &egui::Context) {
        let app_title = match self.shown {
            Some(App::Word) => self.word.app.window_title().to_owned(),
            Some(App::Pdf) => self.pdf.window_title.clone(),
            None => return,
        };
        if app_title.is_empty() {
            return;
        }
        let title = suite_title(&app_title);
        // The app sends its title when it changes; this one goes after it, so it wins.
        if title != self.title || app_title != self.app_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
            self.app_title = app_title;
        }
    }

    fn bar_state(&self) -> bar::State<'_> {
        let pdf_name = self.pdf.active.and_then(|i| self.pdf.views.get(i)).and_then(|v| self.pdf.session.get(v.id)).map(|d| d.display_name());
        bar::State {
            lang: wordcraft_ui_egui::i18n::Lang::from_pref(&self.word.app.ui.language).code(),
            shown: self.shown.unwrap_or(self.want),
            word_name: self.word.app.title_stem(),
            word_dirty: self.word.app.session.dirty,
            pdf_name,
            pdf_count: self.pdf.views.len(),
            pdf_dirty: self.pdf.first_dirty().is_some(),
            live_sync: self.word.app.session.pdf_link.is_some(),
        }
    }

    /// Do what a click in the suite bar asks.
    pub fn act(&mut self, action: bar::Action) {
        match action {
            bar::Action::Show(app) => self.want = app,
            bar::Action::OpenAsPdf => {
                if let Err(e) = self.word.app.run("file.openInPdfCraft", json!({})) {
                    self.word.app.status(e);
                }
            }
            bar::Action::LiveSync(on) => {
                if let Err(e) = self.word.app.run("file.pdfSync", json!({"value": on})) {
                    self.word.app.status(e);
                }
            }
            bar::Action::EditInWord => self.pdf.edit_in_wordcraft(),
        }
    }
}

impl eframe::App for Suite {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        // Ctrl+Alt+PageUp / PageDown (⌘⌥ on macOS) switch app; neither app sees them.
        let before = raw.events.len();
        raw.events.retain(
            |e| !matches!(e, egui::Event::Key { key: egui::Key::PageUp | egui::Key::PageDown, modifiers, .. } if modifiers.command && modifiers.alt),
        );
        if raw.events.len() != before {
            self.want = match self.shown.unwrap_or(self.want) {
                App::Word => App::Pdf,
                App::Pdf => App::Word,
            };
        }
        // A PDF dropped on WordCraft goes to PdfCraft, a Word document dropped on PdfCraft to
        // WordCraft; anything else is the app in view's to handle.
        let shown = self.shown.unwrap_or(self.want);
        let mut elsewhere = Vec::new();
        raw.dropped_files.retain(|f| {
            let path = f.path().to_string_lossy().into_owned();
            if path.is_empty() {
                return true;
            }
            match route::owner(&path) {
                Some(app) if app != shown => {
                    elsewhere.push(path);
                    false
                }
                _ => true,
            }
        });
        self.pending.extend(elsewhere);
        match self.shown {
            Some(App::Word) => self.word.app.raw_input_hook(raw),
            Some(App::Pdf) => eframe::App::raw_input_hook(&mut self.pdf, ctx, raw),
            None => {}
        }
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        if let Some(poll) = self.os_events.as_mut() {
            for event in poll() {
                match event {
                    pdfcraft_ui_egui::OsEvent::Open(paths) => self.pending.extend(paths),
                    pdfcraft_ui_egui::OsEvent::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
            }
        }
        for path in std::mem::take(&mut self.pending) {
            self.open(&path);
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.may_close(ctx);
        }
        self.switch(ctx);
        match self.shown {
            Some(App::Word) => self.word.logic(ctx, frame),
            Some(App::Pdf) => eframe::App::logic(&mut self.pdf, ctx, frame),
            None => {}
        }
        self.word_closed(ctx);
        self.take_handoffs();
        if self.shown != Some(self.want) {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let state = self.bar_state();
        if let Some(action) = bar::show(ui, &state) {
            self.act(action);
            ui.ctx().request_repaint();
        }
        match self.shown {
            Some(App::Word) => self.word.ui(ui, frame),
            Some(App::Pdf) => eframe::App::ui(&mut self.pdf, ui, frame),
            None => {}
        }
        self.sync_title(ui.ctx());
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::App::save(&mut self.pdf, storage);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.word.on_exit();
    }
}

/// WordCraft's File › Export › Open in PdfCraft: the PDF goes to the PDF tab.
fn word_hook(tx: Sender<Handoff>) -> wordcraft::LaunchAppFn {
    Box::new(move |app: &str, path: &str| {
        if app != "pdfcraft" {
            return Err(format!("{app} isn't part of Oldric Suite"));
        }
        tx.send(Handoff::ShowPdf(path.to_owned())).map_err(|e| e.to_string())
    })
}

/// PdfCraft's File ▸ Edit in WordCraft: the Word document goes to the Word tab.
fn pdf_hook(tx: Sender<Handoff>) -> pdfcraft_ui_egui::WordHostFn {
    Box::new(move |path: &std::path::Path| tx.send(Handoff::EditWord(path.to_string_lossy().into_owned())).map_err(|e| e.to_string()))
}

/// `Report - WordCraft` and `Report.pdf — PdfCraft` become `Report — Oldric`.
fn suite_title(app_title: &str) -> String {
    // Just the app's name: nothing open.
    if app_title == "PdfCraft" || app_title == "WordCraft" {
        return "Oldric".to_owned();
    }
    let name = ["- WordCraft", "— PdfCraft"]
        .iter()
        .find_map(|suffix| app_title.strip_suffix(suffix))
        .map(str::trim_end)
        .filter(|name| !name.is_empty())
        .unwrap_or(app_title);
    format!("{name} — Oldric")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_is_named_for_the_suite() {
        assert_eq!(suite_title("Report - WordCraft"), "Report — Oldric");
        assert_eq!(suite_title("Report • - WordCraft"), "Report • — Oldric");
        assert_eq!(suite_title("Report.pdf — PdfCraft"), "Report.pdf — Oldric");
        assert_eq!(suite_title("PdfCraft"), "Oldric");
        assert_eq!(suite_title("Something else"), "Something else — Oldric");
    }

    #[test]
    fn hooks_hand_documents_over_and_refuse_other_apps() {
        let (tx, rx) = channel();
        let word = word_hook(tx.clone());
        word("pdfcraft", "/tmp/a.pdf").unwrap();
        assert!(word("photocraft", "/tmp/a.png").is_err());
        let pdf = pdf_hook(tx);
        pdf(std::path::Path::new("/tmp/b.docx")).unwrap();
        assert_eq!(rx.try_recv().ok(), Some(Handoff::ShowPdf("/tmp/a.pdf".into())));
        assert_eq!(rx.try_recv().ok(), Some(Handoff::EditWord("/tmp/b.docx".into())));
        assert!(rx.try_recv().is_err());
        // The window gone: the hooks report it rather than panic.
        drop(rx);
        assert!(word("pdfcraft", "/tmp/a.pdf").is_err());
    }
}
