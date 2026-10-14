//! Which app a file belongs to.

use std::path::Path;

/// The two apps of the suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    /// WordCraft: documents.
    Word,
    /// PdfCraft: PDFs.
    Pdf,
}

/// Documents only WordCraft opens.
const WORD_ONLY: &[&str] = &["docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "ott", "rtf", "wpd"];

/// Files only PdfCraft opens.
const PDF_ONLY: &[&str] = &["pdf"];

/// Pictures, which PdfCraft turns into PDF pages when they are opened (WordCraft inserts a
/// picture dropped on its page instead).
const PICTURES: &[&str] = &["png", "jpg", "jpeg", "tif", "tiff", "bmp", "gif", "webp"];

fn extension(path: &str) -> String {
    Path::new(path).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// The app that opens `path` from the command line or the system: PdfCraft for PDFs and
/// pictures, WordCraft for everything else (Word and OpenDocument files, RTF, text, Markdown,
/// HTML…).
pub fn opener(path: &str) -> App {
    let ext = extension(path);
    if PDF_ONLY.contains(&ext.as_str()) || PICTURES.contains(&ext.as_str()) { App::Pdf } else { App::Word }
}

/// The app a file dropped on the window must go to whichever app is showing, if any: PDFs to
/// PdfCraft, Word documents to WordCraft. Other files stay with the app they were dropped on,
/// which knows what to do with them (a picture on a WordCraft page is inserted, on PdfCraft's
/// page grid it becomes a page).
pub fn owner(path: &str) -> Option<App> {
    let ext = extension(path);
    if PDF_ONLY.contains(&ext.as_str()) {
        Some(App::Pdf)
    } else if WORD_ONLY.contains(&ext.as_str()) {
        Some(App::Word)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_open_in_the_app_that_edits_them() {
        for pdf in ["Report.pdf", "/tmp/a/REPORT.PDF", "scan.jpeg", "C:\\x\\photo.PNG"] {
            assert_eq!(opener(pdf), App::Pdf, "{pdf}");
        }
        for word in ["Letter.docx", "old.doc", "notes.odt", "x.rtf", "readme.md", "page.html", "plain.txt", "no-extension", ""] {
            assert_eq!(opener(word), App::Word, "{word}");
        }
    }

    #[test]
    fn only_unambiguous_drops_change_app() {
        assert_eq!(owner("a.pdf"), Some(App::Pdf));
        assert_eq!(owner("/x/Letter.DOCX"), Some(App::Word));
        assert_eq!(owner("b.odt"), Some(App::Word));
        // Pictures, fonts, spreadsheets and text stay with the app they were dropped on.
        for other in ["photo.png", "font.ttf", "list.xlsx", "notes.txt", "pdf", ".pdf.png"] {
            assert_eq!(owner(other), None, "{other}");
        }
    }
}
