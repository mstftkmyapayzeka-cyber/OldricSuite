//! The Craft suite: File ▸ Edit in WordCraft. The PDF is exported to Word (the same export as
//! File ▸ Export to Word) and opened in WordCraft, the ArtCraft word processor, for editing.
//! WordCraft's File ▸ Export ▸ Open in PdfCraft brings it back as a PDF that PdfCraft reloads
//! whenever it changes (`file_watch`).
//!
//! The Word document is never written over an existing file: it gets a free name beside the PDF
//! (`Report.docx`, `Report (2).docx`, …), or in the temporary folder for a PDF with no file.
//!
//! WordCraft is looked for, in order: the `WORDCRAFT_BIN` environment variable, next to this
//! executable, then on `PATH`; on macOS an installed `WordCraft.app` through `open -a`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::PdfCraftApp;

/// How many numbered names (`Name (2).docx` …) are tried before giving up.
const MAX_NAME_TRIES: u32 = 99;

impl PdfCraftApp {
    /// File ▸ Edit in WordCraft.
    pub fn edit_in_wordcraft(&mut self) {
        // What's typed in a form field is part of the document (#166).
        if !self.commit_form_typing() {
            return;
        }
        let Some((_, id)) = self.active_ids() else { return };
        if !self.may_copy(id) {
            return;
        }
        let Some(doc) = self.session.get(id) else { return };
        let stem = doc.name.trim_end_matches(".pdf").trim_end_matches(".PDF").to_string();
        let beside = doc.path.as_ref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
        let bytes = doc.export_office(pdfcraft_engine::compare::OfficeFormat::Docx);
        let target = match self.save_override.clone() {
            Some(p) => Ok(PathBuf::from(p)),
            None => docx_path(beside, &stem),
        };
        let path = match target {
            Ok(p) => p,
            Err(e) => return self.notify_error(e),
        };
        let shown = path.display().to_string();
        if let Err(e) = crate::editing::write_atomically(&path.to_string_lossy(), &bytes) {
            return self.notify_fmt("Couldn't write {name}: {e}", &[("name", &shown), ("e", &e.to_string())]);
        }
        match launch(&path) {
            Ok(()) => self.notify_fmt("Opened in WordCraft: {path}", &[("path", &shown)]),
            Err(e) => {
                log::warn!("starting WordCraft for {shown}: {e}");
                self.notify_fmt("WordCraft couldn't be started ({e}). The Word document is at {path}", &[("e", &e), ("path", &shown)]);
            }
        }
    }
}

/// A `.docx` path no file has yet: beside the PDF, else in the temporary folder.
fn docx_path(beside: Option<PathBuf>, stem: &str) -> Result<PathBuf, String> {
    let dir = match beside {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => {
            let d = std::env::temp_dir().join("pdfcraft-docx");
            std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
            d
        }
    };
    let stem = safe_stem(stem);
    let stem = if stem.is_empty() { "Document".to_string() } else { stem };
    free_name(&dir, &stem, |p| p.exists()).ok_or_else(|| format!("no free name for {stem}.docx in {}", dir.display()))
}

/// `<dir>/<stem>.docx`, else the first free `<dir>/<stem> (n).docx`.
fn free_name(dir: &Path, stem: &str, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let first = dir.join(format!("{stem}.docx"));
    if !exists(&first) {
        return Some(first);
    }
    (2..=MAX_NAME_TRIES).map(|n| dir.join(format!("{stem} ({n}).docx"))).find(|p| !exists(p))
}

/// A document name made safe as a file name: separators and characters Windows refuses become
/// spaces, control characters go, and a name of dots can't climb out of the folder.
fn safe_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { ' ' } else { c })
        .take(120)
        .collect();
    cleaned.trim().trim_matches('.').trim().to_string()
}

/// Start WordCraft with the file. The child runs on its own; a thread reaps it when it exits.
fn launch(file: &Path) -> Result<(), String> {
    let mut cmd = match find_executable() {
        Some(exe) => {
            let mut c = Command::new(exe);
            c.arg(file);
            c
        }
        None if cfg!(target_os = "macos") => {
            let mut c = Command::new("open");
            c.arg("-a").arg("WordCraft").arg(file);
            c
        }
        None => return Err("wordcraft wasn't found: install it, put it next to PdfCraft or set WORDCRAFT_BIN".into()),
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn find_executable() -> Option<PathBuf> {
    let name = if cfg!(windows) { "wordcraft.exe" } else { "wordcraft" };
    let env = std::env::var_os("WORDCRAFT_BIN").map(PathBuf::from);
    let beside = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join(name)));
    let on_path = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).map(|d| d.join(name)).collect::<Vec<_>>()).unwrap_or_default();
    pick(env, beside, on_path, |p| p.is_file())
}

/// The first candidate that is a file: the configured one, the one beside us, then `PATH`'s.
fn pick(env: Option<PathBuf>, beside: Option<PathBuf>, on_path: Vec<PathBuf>, is_file: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    env.into_iter().chain(beside).chain(on_path).find(|p| is_file(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_name_never_overwrites() {
        let dir = Path::new("/docs");
        assert_eq!(free_name(dir, "Report", |_| false), Some(dir.join("Report.docx")));
        let taken = [dir.join("Report.docx")];
        assert_eq!(free_name(dir, "Report", |p| taken.iter().any(|t| t == p)), Some(dir.join("Report (2).docx")));
        assert_eq!(free_name(dir, "Report", |_| true), None);
    }

    #[test]
    fn names_are_made_safe() {
        assert_eq!(safe_stem("a/b:c"), "a b c");
        assert_eq!(safe_stem(".."), "");
        assert_eq!(safe_stem("x\u{7}y"), "xy");
    }

    #[test]
    fn the_configured_binary_wins_then_the_neighbour_then_path() {
        let all = |_: &Path| true;
        let env = Some(PathBuf::from("/opt/wordcraft"));
        let beside = Some(PathBuf::from("/apps/wordcraft"));
        let path = vec![PathBuf::from("/usr/bin/wordcraft")];
        assert_eq!(pick(env.clone(), beside.clone(), path.clone(), all), env);
        assert_eq!(pick(None, beside.clone(), path.clone(), all), beside);
        assert_eq!(pick(None, None, path.clone(), |p| p == Path::new("/usr/bin/wordcraft")), path.first().cloned());
    }

    #[test]
    fn a_pdf_without_a_file_exports_to_the_temporary_folder() {
        let p = docx_path(None, "Scan").unwrap();
        assert!(p.starts_with(std::env::temp_dir().join("pdfcraft-docx")));
        assert!(p.to_string_lossy().ends_with(".docx"));
    }
}
