//! The suite's own interface text, in the language WordCraft's interface uses (File › Options ›
//! Language). English and Turkish; other languages read English until they get a column here.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    /// The WordCraft tab.
    Word,
    /// The PdfCraft tab.
    Pdf,
    /// The PdfCraft tab with no document open.
    NoPdf,
    /// The dot after a name with unsaved changes.
    Unsaved,
    /// WordCraft: File › Export › Open in PdfCraft.
    OpenAsPdf,
    OpenAsPdfTip,
    /// WordCraft: Keep PDF in Sync.
    LiveSync,
    LiveSyncTip,
    /// PdfCraft: File ▸ Edit in WordCraft.
    EditInWord,
    EditInWordTip,
    /// Switching tabs.
    SwitchTip,
}

pub fn tr(lang: &str, text: Text) -> &'static str {
    let turkish = lang.eq_ignore_ascii_case("tr");
    match (text, turkish) {
        (Text::Word, _) => "Word",
        (Text::Pdf, _) => "PDF",
        (Text::NoPdf, false) => "No PDF open",
        (Text::NoPdf, true) => "Açık PDF yok",
        (Text::Unsaved, false) => "Unsaved changes",
        (Text::Unsaved, true) => "Kaydedilmemiş değişiklikler",
        (Text::OpenAsPdf, false) => "Open as PDF",
        (Text::OpenAsPdf, true) => "PDF olarak aç",
        (Text::OpenAsPdfTip, false) => "Write the document as a PDF beside it and show it in the PDF tab",
        (Text::OpenAsPdfTip, true) => "Belgeyi yanına PDF olarak yazar ve PDF sekmesinde gösterir",
        (Text::LiveSync, false) => "Live sync",
        (Text::LiveSync, true) => "Canlı eşitleme",
        (Text::LiveSyncTip, false) => "While on, the PDF is written again after every change, and the PDF tab shows it",
        (Text::LiveSyncTip, true) => "Açıkken PDF her değişiklikten sonra yeniden yazılır ve PDF sekmesi onu gösterir",
        (Text::EditInWord, false) => "Edit in Word",
        (Text::EditInWord, true) => "Word'de düzenle",
        (Text::EditInWordTip, false) => "Turn the PDF into a Word document beside it and open it in the Word tab",
        (Text::EditInWordTip, true) => "PDF'yi yanına Word belgesi olarak dönüştürür ve Word sekmesinde açar",
        (Text::SwitchTip, false) => "Switch between the document and the PDFs (Ctrl+Alt+PageUp / PageDown)",
        (Text::SwitchTip, true) => "Belge ile PDF'ler arasında geç (Ctrl+Alt+PageUp / PageDown)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_has_its_own_words_and_others_read_english() {
        assert_eq!(tr("tr", Text::OpenAsPdf), "PDF olarak aç");
        assert_eq!(tr("TR", Text::EditInWord), "Word'de düzenle");
        assert_eq!(tr("en", Text::OpenAsPdf), "Open as PDF");
        assert_eq!(tr("de", Text::LiveSync), "Live sync");
        assert_eq!(tr("", Text::NoPdf), "No PDF open");
    }
}
