# Oldric Suite

**Word documents and PDFs in one app.** Oldric Suite brings together two open-source projects
written in Rust, both by the ArtCraft team. WordCraft is a word processor in the spirit of
Microsoft Word, and PdfCraft is a PDF editor in the spirit of Adobe Acrobat. Both run in one
window, and each keeps every feature it has on its own.

*Türkçe açıklama aşağıda: [Türkçe](#türkçe).*

## What it does

- **One window, two tabs.** The bar along the top has a **Word** tab, with the document you are
  writing, and a **PDF** tab, with your open PDFs. Click a tab, or press Ctrl+Alt+PageUp /
  PageDown (⌘⌥ on macOS), to switch. Each app keeps its own state while the other one is in
  view.
- **Word → PDF.** *Open as PDF* writes the document as a PDF beside it and shows it in the PDF
  tab. There you can annotate, sign, organize, protect, run OCR and use every other PdfCraft
  tool. WordCraft's File › Export › Open in PdfCraft does the same.
- **Live sync.** With *Live sync* on, the PDF is written again after every save and after a
  short pause in typing. Switching to the PDF tab also writes the PDF at once, so the PDF tab
  always shows the document as it is now.
- **PDF → Word.** *Edit in Word* turns the PDF into a Word document beside it and opens it in
  the Word tab. PdfCraft's File ▸ Edit in WordCraft does the same.
- **Files go to the right app.** On the command line, `oldric report.docx scan.pdf` opens each
  file in its own tab. When you drop a PDF on the Word tab, it opens in the PDF tab, and a Word
  document dropped on the PDF tab opens in the Word tab. A picture dropped on a Word page is
  inserted, as it is in WordCraft.

## Build and run

You need Rust 1.95 or later.

```sh
cargo run --release -p oldric                         # the suite
cargo run --release -p oldric -- report.docx a.pdf    # open files in it
```

The two apps still build on their own, along with their command-line tools:

```sh
cargo run --release -p wordcraft -- --sample
cargo run --release -p pdfcraft -- some.pdf
cargo run --release -p wordcraft-cli -- convert report.docx report.pdf
cargo run --release -p pdfcraft-cli -- tools
```

For a quick check of the window without a display, run `cargo run -p oldric --example shot --
<out-dir> [lang]`. It renders the window offscreen and walks through Word → PDF → Live sync →
PDF → Word, saving a screenshot at each step.

### Settings

- WordCraft keeps its settings where the `wordcraft` program does, so both share them.
- PdfCraft's settings, and the suite's window size and position, are stored in the suite's own
  settings folder, in `app.ron`. On the first run, that file starts as a copy of PdfCraft's.
- The log files are in the `logs` folder inside that settings folder.
- Set `OLDRIC_RENDERER=gl` to draw with OpenGL instead of wgpu.
- On Linux, set `OLDRIC_WAYLAND=1` to keep native Wayland. By default the suite runs through
  XWayland, because winit supports file drag-and-drop only on X11.

## Layout

| Folder | What it is |
|---|---|
| `apps/oldric` | The suite: the window, the bar and how the two apps hand documents to each other |
| `wordcraft/` | WordCraft, complete: its engine, formats, interface, docs, tests and tools |
| `pdfcraft/` | PdfCraft, complete: its engine, renderer, interface, docs, tests, tools and patched crates (`vendor/`) |

The repository is a single Cargo workspace. PdfCraft's patched copies of upstream crates (hayro,
lopdf, winit and egui-wgpu; see `pdfcraft/vendor/README.md`) apply to WordCraft too, since it uses
the same versions. Each project's own tools run with `cargo xtask-word …` and `cargo xtask-pdf …`.
The rules for working in each folder (no panics, clean-room work, asset licences) are in that
folder's `AGENTS.md`/`CLAUDE.md`. They apply here unchanged.

## Not yet

- **One Word document at a time.** WordCraft edits one document per window. Opening another one
  replaces it, and asks first about unsaved changes. The PDF tab holds any number of PDFs.
- **Changes don't flow back from the PDF.** Comments or signatures added in the PDF tab stay in
  the PDF. While Live sync is on, the PDF is rewritten from the Word document, so do PDF-side work
  on a copy (Save As in the PDF tab), or turn Live sync off.
- **Desktop only.** Each project still has its own web build (`wordcraft/apps/wordcraft-web`,
  `pdfcraft/apps/pdfcraft-web`), but the suite has no web build yet.
- **No installers yet.** The suite has no installer or package of its own. The projects'
  `packaging/` folders build the separate apps.
- **Native title bar.** The window uses the system title bar on every platform. The apps' own
  title-bar layouts for macOS and GNOME aren't used in the suite.

## Updating from the original projects

`wordcraft/` and `pdfcraft/` were imported from
[Oldricword](https://github.com/mstftkmyapayzeka-cyber/Oldricword) at `5d2a086` and
[OldricPdf](https://github.com/mstftkmyapayzeka-cyber/OldricPdf) at `ddbbeac`. The first commit of
this repository records both. To bring in later changes from one of them, apply that project's
diff inside its folder, for example:

```sh
git -C ../Oldricword diff 5d2a086 main | git apply --directory=wordcraft -3
```

Changes made here to fit both projects into one workspace are likely to conflict with upstream
changes to the same files. These are the projects' `Cargo.toml` files, where versions are now
written out instead of inherited, and the app crates, which are now libraries as well. Resolve the
conflicts by keeping both sides.

## License

MIT OR Apache-2.0, at your option ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
Third-party material and trademarks: [NOTICE](NOTICE).

---

## Türkçe

**Word belgeleri ve PDF'ler tek uygulamada.** Oldric Suite, ArtCraft ekibinin Rust ile yazdığı iki
açık kaynak projeyi bir araya getirir. WordCraft, Microsoft Word benzeri bir kelime işlemcidir;
PdfCraft ise Adobe Acrobat benzeri bir PDF düzenleyicidir. İkisi aynı pencerede çalışır ve her
biri tek başına sahip olduğu tüm özellikleri korur.

- **Tek pencere, iki sekme.** Üstteki çubukta **Word** sekmesi (yazdığınız belge) ve **PDF**
  sekmesi (açık PDF'leriniz) bulunur. Sekmeye tıklayarak ya da Ctrl+Alt+PageUp / PageDown ile
  geçiş yapılır. Arka plandaki uygulama kaldığı yerden devam eder.
- **Word → PDF.** *PDF olarak aç*, belgeyi yanına PDF olarak yazar ve PDF sekmesinde gösterir.
  Orada not ekleyebilir, imzalayabilir, sayfaları düzenleyebilir, şifreleyebilir, OCR
  uygulayabilir ve PdfCraft'ın diğer tüm araçlarını kullanabilirsiniz.
- **Canlı eşitleme.** Açıkken PDF her kaydetmede ve yazmaya kısa bir ara verildiğinde yeniden
  yazılır. PDF sekmesine geçildiğinde de bekleyen değişiklik hemen yazılır. Böylece PDF sekmesi
  her zaman belgenin güncel hâlini gösterir.
- **PDF → Word.** *Word'de düzenle*, PDF'yi yanına Word belgesi olarak dönüştürür ve Word
  sekmesinde açar.
- **Dosyalar doğru uygulamaya gider.** `oldric rapor.docx tarama.pdf` her dosyayı kendi
  sekmesinde açar. Word sekmesine bırakılan bir PDF, PDF sekmesinde açılır. PDF sekmesine
  bırakılan bir Word belgesi de Word sekmesinde açılır.

Derlemek ve çalıştırmak için Rust 1.95 veya üstü gerekir:

```sh
cargo run --release -p oldric
cargo run --release -p oldric -- rapor.docx belge.pdf
```

Arayüz dili WordCraft'ın dil ayarını izler (Dosya › Seçenekler › Dil). Türkçe seçiliyse çubuk da
Türkçedir. Henüz olmayanlar yukarıdaki *Not yet* bölümünde listelenmiştir: aynı anda tek Word
belgesi, PDF'de yapılan değişikliklerin Word'e geri aktarılmaması ve henüz kurulum paketi
olmaması.
