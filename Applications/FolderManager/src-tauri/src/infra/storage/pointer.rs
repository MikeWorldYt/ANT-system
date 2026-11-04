// Remembers the Storage of each copy of the executable in a file outside the Storage, since the app has no data of its own.
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::{AppHandle, Manager};

use super::db::hash;

const DIR: &str = "storage-pointers";

#[derive(Deserialize)]
struct Pointer {
    path: PathBuf,
}

fn parse(text: &str) -> Option<PathBuf> {
    let pointer: Pointer = serde_yaml::from_str(text).ok()?;
    (!pointer.path.as_os_str().is_empty()).then_some(pointer.path)
}

fn render(root: &Path) -> String {
    let text = root.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    format!("{{\"path\": \"{text}\"}}\n")
}

pub fn file(data_dir: &Path, exe: &Path) -> PathBuf {
    data_dir.join(DIR).join(format!("{}.json", hash(&exe.to_string_lossy())))
}

pub fn read(file: &Path) -> Option<PathBuf> {
    parse(&fs::read_to_string(file).ok()?)
}

pub fn write(file: &Path, root: &Path) -> std::io::Result<()> {
    if read(file).as_deref() == Some(root) {
        return Ok(());
    }
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file, render(root))
}

pub fn app_file(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(file(&data_dir, &exe))
}

// A pointer that cannot be saved only costs the next launch its memory, so the caller's flow goes on.
pub fn remember(app: &AppHandle, root: &Path) {
    if let Err(e) = app_file(app).and_then(|f| write(&f, root).map_err(|e| e.to_string())) {
        eprintln!("Could not remember the Storage path: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_pointer_reads_back() {
        let dir = TempDir::new("pointer");
        let f = file(dir.path(), Path::new("C:\\Apps\\fm.exe"));
        assert!(f.starts_with(dir.path().join(DIR)));
        assert_ne!(f, file(dir.path(), Path::new("C:\\Other\\fm.exe")));
        let root = Path::new("C:\\Users\\Me\\My \"Storage\"");
        write(&f, root).unwrap();
        assert_eq!(read(&f).as_deref(), Some(root));
    }

    #[test]
    fn a_corrupt_pointer_is_ignored() {
        let dir = TempDir::new("pointer-bad");
        let f = dir.path().join("p.json");
        assert_eq!(read(&f), None);
        for text in ["not json", "{\"path\": \"\"}", "{\"other\": 1}", "[1, 2]"] {
            fs::write(&f, text).unwrap();
            assert_eq!(read(&f), None, "{text}");
        }
    }
}
