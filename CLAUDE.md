# Oldric Suite: instructions for agents

This repository combines two projects into one Cargo workspace and one app:

- `wordcraft/`: WordCraft, a word processor (Microsoft Word parity).
- `pdfcraft/`: PdfCraft, a PDF editor (Adobe Acrobat parity).
- `apps/oldric/`: the suite, which runs both in one window (`src/shell.rs`), with a bar to switch
  between them and hand documents across (`src/bar.rs`).

## Start here

- Before you change anything inside `wordcraft/` or `pdfcraft/`, read that folder's `CLAUDE.md`
  and `AGENTS.md`. Their rules apply unchanged to their folders: never crash, clean-room work only,
  no Microsoft or Adobe assets, an attribution row for every asset, and their layering.
- The suite follows the same rules:
  - No panics in non-test code: no `unwrap`, `expect`, `panic!` or indexing with input-derived
    positions, and no `unsafe`.
  - The suite's own art is drawn in code (`icon.rs`, `bar.rs`).

## How the two apps share the window

- Only the app in view runs. Each frame the shell calls that app's `logic` and `ui`; the other
  app waits with its state kept.
- Both apps configure egui themselves: fonts, theme, zoom and title. When an app comes into view,
  the shell calls its `reclaim_context()` (`WordApp`, `PdfCraftApp`). That method reinstalls the
  app's look and skips drawing until its fonts are loaded.
  - If you add egui-wide state to either app (`ctx.set_*`, `options_mut`, `all_styles_mut`) and
    cache it behind an "already applied" flag, reset that flag in `reclaim_context`.
- The apps hand documents to each other through hooks set by the shell:
  - WordCraft's `Services::launch_app` (Open in PdfCraft) sends the PDF to `PdfCraftApp::show_path`.
  - PdfCraft's `PdfCraftApp::word_host` (Edit in WordCraft) sends the `.docx` to WordCraft's
    `file.open`.
  - On the way to the PDF tab, `WordApp::flush_pdf_link` writes any pending Live sync edit.
- Closing the window asks each app about unsaved work in turn (`Suite::may_close`).
  - In the suite, WordCraft's own Close (Ctrl+W) closes the document, not the window
    (`Suite::word_closed`).
- `wordcraft` (`wordcraft/apps/wordcraft`) and `pdfcraft` (`pdfcraft/apps/pdfcraft`) are both
  libraries and programs. The suite uses them as libraries:
  - from `wordcraft`: `Embedded`;
  - from `pdfcraft`: `configure_app`, `native_options`, `logging` and `apple_events`.
  - Keep the programs' behaviour unchanged when you change these.

## Quality gates

- `cargo fmt --all --check`.
- Run clippy with `-D warnings` and the tests, scoped to the crates you touched, under a
  `timeout`. For example: `cargo clippy -p oldric --all-targets -- -D warnings` and
  `cargo test -p oldric`.
  - A full `--workspace` build compiles both projects, and it is CI's job.
- For window work, look at the result:
  `WORDCRAFT_NO_PREFS=1 cargo run -p oldric --example shot -- /tmp/shots tr` renders the window
  offscreen, with no display needed.
  - It needs a Vulkan driver; lavapipe (`mesa-vulkan-drivers`) works on a machine without a GPU.
- Parallel agents should each use their own `CARGO_TARGET_DIR`.
