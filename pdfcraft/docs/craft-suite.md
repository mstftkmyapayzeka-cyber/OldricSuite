# PdfCraft ⇄ WordCraft

Status: first version (2026-10-13). PdfCraft and WordCraft stay two apps with their own engines
and hand documents to each other so they work as one suite.

## PdfCraft → WordCraft

**File ▸ Edit in WordCraft** (`export.wordcraft`) exports the active PDF to Word (the export of
File ▸ Export to Word…; documents that forbid copying refuse it the same way), writes it beside the
PDF under a name no file has (`Report.docx`, `Report (2).docx`, …; the temporary folder for a PDF
with no file) and starts WordCraft with it. The web build downloads the Word document instead.

## WordCraft → PdfCraft

WordCraft's **File › Export › Open in PdfCraft** writes its document as a PDF and starts PdfCraft
with it. With **Keep PDF in Sync** on, WordCraft writes that PDF again on every save and after a
pause in editing; PdfCraft's file watcher (`file_watch`, Preferences ▸ *Reload documents when
their files change on disk*) reloads it, so the PdfCraft window follows the writing.

## Finding WordCraft

`WORDCRAFT_BIN`, then `wordcraft` next to the PdfCraft executable, then on `PATH`, then on macOS
`open -a WordCraft`. When it isn't found, the notice says where the Word document was written.

## Not yet

- One window and one binary for both apps. The workspaces pin the shared `craft-ui` and
  `craft-print` crates at different revisions, and PdfCraft patches `egui-wgpu` and `hayro` with
  vendored copies that would apply to WordCraft too; those need aligning first.
- PDF-side changes (comments, signatures, form data) don't flow back into the Word document; a
  linked PDF is rewritten from WordCraft's document, so keep PDF-side work in a copy.
