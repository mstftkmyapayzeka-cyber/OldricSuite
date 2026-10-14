//! The suite bar along the top of the window: a tab for each app, and what goes between them for
//! the app in view (WordCraft: Open as PDF and Live sync; PdfCraft: Edit in Word).

use egui::{Align, Color32, CornerRadius, FontId, Layout, Rect, Sense, Stroke, StrokeKind, Ui, vec2};

use crate::route::App;
use crate::text::{Text, tr};

/// The bar's height in points.
pub const HEIGHT: f32 = 34.0;

/// WordCraft's mark colour: a plain blue dot, no product iconography.
const WORD_DOT: Color32 = Color32::from_rgb(0x3B, 0x6F, 0xD8);
/// PdfCraft's: a plain red dot.
const PDF_DOT: Color32 = Color32::from_rgb(0xD9, 0x4A, 0x3D);

/// What the bar shows; read from the two apps before it is drawn.
pub struct State<'a> {
    pub lang: &'a str,
    pub shown: App,
    /// WordCraft's document name and whether it has unsaved changes.
    pub word_name: String,
    pub word_dirty: bool,
    /// PdfCraft's active document name (`None` with no PDF open), how many are open, and whether
    /// any has unsaved changes.
    pub pdf_name: Option<String>,
    pub pdf_count: usize,
    pub pdf_dirty: bool,
    /// Whether WordCraft keeps a PDF in step with the document (Keep PDF in Sync).
    pub live_sync: bool,
}

/// What the user asked for in the bar this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Show(App),
    OpenAsPdf,
    LiveSync(bool),
    EditInWord,
}

struct Palette {
    bar: Color32,
    line: Color32,
    tab: Color32,
    hover: Color32,
    text: Color32,
    faint: Color32,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bar: Color32::from_rgb(0x18, 0x1A, 0x1F),
            line: Color32::from_rgb(0x30, 0x33, 0x3A),
            tab: Color32::from_rgb(0x2C, 0x2F, 0x36),
            hover: Color32::from_rgb(0x24, 0x27, 0x2D),
            text: Color32::from_rgb(0xE8, 0xEA, 0xED),
            faint: Color32::from_rgb(0x9A, 0xA0, 0xA8),
        }
    } else {
        Palette {
            bar: Color32::from_rgb(0xE9, 0xEC, 0xF0),
            line: Color32::from_rgb(0xD0, 0xD5, 0xDC),
            tab: Color32::WHITE,
            hover: Color32::from_rgb(0xF5, 0xF6, 0xF8),
            text: Color32::from_rgb(0x1F, 0x23, 0x28),
            faint: Color32::from_rgb(0x5F, 0x66, 0x70),
        }
    }
}

/// Draw the bar at the top of `ui`; returns what was clicked.
pub fn show(ui: &mut Ui, state: &State<'_>) -> Option<Action> {
    let p = palette(ui.visuals().dark_mode);
    let frame = egui::Frame::NONE.fill(p.bar).inner_margin(egui::Margin::symmetric(8, 0));
    let mut action = None;
    egui::Panel::top("oldric.suite_bar").exact_size(HEIGHT).resizable(false).show_separator_line(false).frame(frame).show(ui, |ui| {
        let full = ui.max_rect();
        ui.painter().hline(full.x_range(), full.bottom() - 0.5, Stroke::new(1.0, p.line));
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            mark(ui, &p);
            ui.add_space(10.0);
            let word_label = format!("{} · {}", tr(state.lang, Text::Word), state.word_name);
            if tab(ui, &p, &word_label, WORD_DOT, state.shown == App::Word, state.word_dirty, state.lang).clicked() {
                action = Some(Action::Show(App::Word));
            }
            let pdf_label = match (&state.pdf_name, state.pdf_count) {
                (Some(name), n) if n > 1 => format!("{} · {name} (+{})", tr(state.lang, Text::Pdf), n - 1),
                (Some(name), _) => format!("{} · {name}", tr(state.lang, Text::Pdf)),
                (None, _) => format!("{} · {}", tr(state.lang, Text::Pdf), tr(state.lang, Text::NoPdf)),
            };
            if tab(ui, &p, &pdf_label, PDF_DOT, state.shown == App::Pdf, state.pdf_dirty, state.lang).clicked() {
                action = Some(Action::Show(App::Pdf));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| match state.shown {
                App::Word => {
                    let sync = toggle(ui, &p, tr(state.lang, Text::LiveSync), state.live_sync).on_hover_text(tr(state.lang, Text::LiveSyncTip));
                    if sync.clicked() {
                        action = Some(Action::LiveSync(!state.live_sync));
                    }
                    if button(ui, &p, tr(state.lang, Text::OpenAsPdf), true).on_hover_text(tr(state.lang, Text::OpenAsPdfTip)).clicked() {
                        action = Some(Action::OpenAsPdf);
                    }
                }
                App::Pdf => {
                    let open = state.pdf_name.is_some();
                    if button(ui, &p, tr(state.lang, Text::EditInWord), open).on_hover_text(tr(state.lang, Text::EditInWordTip)).clicked() {
                        action = Some(Action::EditInWord);
                    }
                }
            });
        });
    });
    action
}

/// The suite's mark: two overlapping pages, drawn here (no image asset).
fn mark(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
    let back = Rect::from_min_size(rect.min + vec2(1.0, 1.5), vec2(12.0, 15.0));
    let front = Rect::from_min_size(rect.min + vec2(8.5, 5.5), vec2(12.0, 15.0));
    ui.painter().rect_filled(back, CornerRadius::same(2), WORD_DOT);
    ui.painter().rect(front, CornerRadius::same(2), PDF_DOT, Stroke::new(1.5, p.bar), StrokeKind::Outside);
}

/// One app's tab: a dot in the app's colour, the label, and a dot for unsaved changes.
fn tab(ui: &mut Ui, p: &Palette, label: &str, dot: Color32, selected: bool, dirty: bool, lang: &str) -> egui::Response {
    let color = if selected { p.text } else { p.faint };
    let galley = ui.painter().layout_no_wrap(truncate(label, 40), FontId::proportional(13.0), color);
    let width = 12.0 + 8.0 + 8.0 + galley.size().x + if dirty { 14.0 } else { 0.0 } + 12.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 26.0), Sense::click());
    let response = response.on_hover_text(tr(lang, Text::SwitchTip));
    let painter = ui.painter();
    if selected {
        painter.rect(rect, CornerRadius::same(6), p.tab, Stroke::new(1.0, p.line), StrokeKind::Inside);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(6), p.hover);
    }
    let center_y = rect.center().y;
    painter.circle_filled(egui::pos2(rect.left() + 16.0, center_y), 4.0, dot);
    let text_pos = egui::pos2(rect.left() + 28.0, center_y - galley.size().y / 2.0);
    let text_width = galley.size().x;
    painter.galley(text_pos, galley, color);
    if dirty {
        let at = egui::pos2(text_pos.x + text_width + 8.0, center_y);
        painter.circle_filled(at, 3.0, p.faint);
    }
    if dirty { response.on_hover_text(tr(lang, Text::Unsaved)) } else { response }
}

/// A plain button in the bar's colours.
fn button(ui: &mut Ui, p: &Palette, label: &str, enabled: bool) -> egui::Response {
    let text = egui::RichText::new(label).font(FontId::proportional(12.5)).color(if enabled { p.text } else { p.faint });
    ui.add_enabled(
        enabled,
        egui::Button::new(text).fill(p.tab).stroke(Stroke::new(1.0, p.line)).corner_radius(CornerRadius::same(6)).min_size(vec2(0.0, 24.0)),
    )
}

/// An on/off switch with its label.
fn toggle(ui: &mut Ui, p: &Palette, label: &str, on: bool) -> egui::Response {
    let font = FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, p.text);
    let (rect, response) = ui.allocate_exact_size(vec2(30.0 + 8.0 + galley.size().x + 4.0, 24.0), Sense::click());
    let painter = ui.painter();
    let track = Rect::from_min_size(egui::pos2(rect.left(), rect.center().y - 8.0), vec2(30.0, 16.0));
    painter.rect(track, CornerRadius::same(8), if on { WORD_DOT } else { p.line }, Stroke::NONE, StrokeKind::Inside);
    let knob_x = if on { track.right() - 8.0 } else { track.left() + 8.0 };
    painter.circle_filled(egui::pos2(knob_x, track.center().y), 6.0, Color32::WHITE);
    painter.galley(egui::pos2(track.right() + 8.0, rect.center().y - galley.size().y / 2.0), galley, p.text);
    response
}

/// `text` cut to `max` characters with an ellipsis.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    #[test]
    fn long_names_are_cut_at_a_character() {
        assert_eq!(super::truncate("short", 36), "short");
        assert_eq!(super::truncate("ğüşiöç ğüşiöç", 5), "ğüşi…");
        assert_eq!(super::truncate("abc", 0), "…");
    }
}
