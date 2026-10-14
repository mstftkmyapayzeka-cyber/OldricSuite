//! Headless screenshots of the whole window (wgpu offscreen, so no display is needed):
//! `cargo run -p oldric --example shot -- <out-dir> [lang]`.
//!
//! Walks through the suite: a report in the Word tab, Open as PDF, Live sync with an edit, then
//! Edit in Word from the PDF tab, saving `01-word.png` … `06-word-from-pdf.png` in `<out-dir>`.
//! Runs without the user's preferences (`WORDCRAFT_NO_PREFS`) and works in a scratch folder.

use egui_kittest::Harness;
use oldric::bar::Action;
use oldric::route::App;
use oldric::shell::Suite;
use serde_json::json;

fn steps(h: &mut Harness<'_, Suite>, n: usize) {
    for _ in 0..n {
        h.step();
    }
}

fn shot(h: &mut Harness<'_, Suite>, dir: &std::path::Path, name: &str) {
    steps(h, 6);
    match h.render() {
        Ok(img) => {
            let path = dir.join(name);
            img.save(&path).expect("save png");
            println!("{}", path.display());
        }
        Err(e) => eprintln!("render failed: {e}"),
    }
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("usage: shot <out-dir> [lang]"));
    let lang = std::env::args().nth(2).unwrap_or_else(|| "en".into());
    std::fs::create_dir_all(&out).expect("out dir");
    let work = std::env::temp_dir().join(format!("oldric-shot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("scratch dir");
    if std::env::var_os("WORDCRAFT_NO_PREFS").is_none() {
        eprintln!("note: WORDCRAFT_NO_PREFS=1 keeps this run off your WordCraft preferences");
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(1_000_000)
        .wgpu()
        .build_eframe(|cc| Suite::new(cc, &[]));
    h.input_mut().max_texture_side = Some(8192);
    {
        let suite = h.state_mut();
        suite.word().ui.language = lang.clone();
        let _ = suite.pdf().set_option("language", &lang);
        suite.word().run("file.new", json!({"template": "report"})).expect("report");
        // As if the Start screen's Report was clicked.
        suite.word().ui.backstage = false;
    }
    steps(&mut h, 10);
    shot(&mut h, &out, "01-word.png");

    let docx = work.join("Report.docx");
    h.state_mut().word().run("file.saveAs", json!({"path": docx.to_string_lossy()})).expect("save as");
    h.state_mut().act(Action::OpenAsPdf);
    steps(&mut h, 20);
    shot(&mut h, &out, "02-pdf.png");

    h.state_mut().act(Action::Show(App::Word));
    h.state_mut().act(Action::LiveSync(true));
    steps(&mut h, 6);
    {
        let word = h.state_mut().word();
        word.run("select.start", json!({})).ok();
        word.run("text.insert", json!({"text": "Live sync: typed in the Word tab. "})).expect("type");
    }
    steps(&mut h, 4);
    shot(&mut h, &out, "03-word-typed.png");
    h.state_mut().act(Action::Show(App::Pdf));
    // PdfCraft notices the rewritten file and reloads it.
    for _ in 0..40 {
        steps(&mut h, 5);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    shot(&mut h, &out, "04-pdf-synced.png");

    h.state_mut().act(Action::EditInWord);
    steps(&mut h, 20);
    // The edited document asks first; this run keeps it ("Don't Save" opens the new one).
    shot(&mut h, &out, "05-save-first.png");
    h.state_mut().word().run("ui.saveChanges", json!({"answer": "dontSave"})).expect("answer");
    steps(&mut h, 20);
    shot(&mut h, &out, "06-word-from-pdf.png");
    let _ = std::fs::remove_dir_all(&work);
}
