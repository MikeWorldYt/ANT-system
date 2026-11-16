// The Storages this install has opened, newest first, so the picker can list them outside any Storage.
use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::infra::fs_safe::write_atomic_io;

const FILE: &str = "storages.json";

fn same(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

pub fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

// A missing or corrupt list only loses the history, so it reads as empty.
pub fn read(file: &Path) -> Vec<PathBuf> {
    fs::read_to_string(file).ok().and_then(|t| serde_json::from_str::<Vec<PathBuf>>(&t).ok()).unwrap_or_default()
}

fn save(file: &Path, list: &[PathBuf]) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    write_atomic_io(file, serde_json::to_string_pretty(list).map_err(std::io::Error::other)?.as_bytes())
}

pub fn touch(file: &Path, root: &Path) -> std::io::Result<()> {
    let mut list = read(file);
    if list.first().is_some_and(|p| same(p, root)) {
        return Ok(());
    }
    list.retain(|p| !same(p, root));
    list.insert(0, root.to_path_buf());
    save(file, &list)
}

pub fn forget(file: &Path, root: &Path) -> std::io::Result<()> {
    let mut list = read(file);
    list.retain(|p| !same(p, root));
    save(file, &list)
}

pub fn replace(file: &Path, old: &Path, new: &Path) -> std::io::Result<()> {
    let list: Vec<PathBuf> = read(file).into_iter().map(|p| if same(&p, old) { new.to_path_buf() } else { p }).collect();
    save(file, &list)
}

pub fn listed(file: &Path, root: &Path) -> bool {
    read(file).iter().any(|p| same(p, root))
}

pub fn app_file(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_local_data_dir().map(|d| file(&d)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn the_list_keeps_the_newest_first_without_duplicates() {
        let dir = TempDir::new("known");
        let f = file(dir.path());
        assert!(read(&f).is_empty());
        touch(&f, Path::new("C:\\A")).unwrap();
        touch(&f, Path::new("C:\\B")).unwrap();
        touch(&f, Path::new("c:\\a")).unwrap();
        assert_eq!(read(&f), vec![PathBuf::from("c:\\a"), PathBuf::from("C:\\B")]);
        replace(&f, Path::new("C:\\B"), Path::new("C:\\C")).unwrap();
        assert!(listed(&f, Path::new("c:\\c")) && !listed(&f, Path::new("C:\\B")));
        forget(&f, Path::new("C:\\C")).unwrap();
        assert_eq!(read(&f), vec![PathBuf::from("c:\\a")]);
        fs::write(&f, "not json").unwrap();
        assert!(read(&f).is_empty());
    }

    #[test]
    fn a_save_replaces_the_list_whole_and_leaves_no_temp_file() {
        let dir = TempDir::new("known-atomic");
        let f = file(dir.path());
        fs::write(&f, "not json").unwrap();
        touch(&f, Path::new("C:\\A")).unwrap();
        assert_eq!(read(&f), vec![PathBuf::from("C:\\A")]);
        let left: Vec<String> = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left, [FILE]);
    }
}
