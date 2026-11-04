// Name lists of 998.Review links (starred, hidden) kept in the app's data folder, because a link has no folder whose info.json could hold the flag.
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::DATA_DIR;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, write_atomic};
use crate::storage::info::json_string;

fn file(root: &Path, file_name: &str) -> PathBuf {
    root.join(DATA_DIR).join(file_name)
}

// A missing or blank file means no link is listed yet.
pub(crate) fn read(root: &Path, file_name: &str) -> R<Vec<String>> {
    let path = file(root, file_name);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err("read the link list", &path, &e)),
    };
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_yaml::from_str(&text)
        .map_err(|e| FmError::Internal { message: format!("{file_name} must hold a list of link names: {e}") })
}

pub(crate) fn set(root: &Path, file_name: &str, link: &str, on: bool) -> R<()> {
    let mut names = read(root, file_name)?;
    names.retain(|name| name != link);
    if on {
        names.push(link.to_string());
    }
    names.sort();
    let items: Vec<String> = names.iter().map(|name| format!("  {}", json_string(name))).collect();
    let text = if items.is_empty() { "[]\n".to_string() } else { format!("[\n{}\n]\n", items.join(",\n")) };
    let dir = root.join(DATA_DIR);
    fs::create_dir_all(&dir).map_err(|e| io_err("create the data folder", &dir, &e))?;
    write_atomic(&file(root, file_name), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LINK_FAVORITES_FILENAME, LINK_HIDDEN_FILENAME};
    use crate::test_support::TempDir;

    #[test]
    fn a_missing_or_blank_file_lists_no_links() {
        let dir = TempDir::new("link-list-empty");
        assert!(read(dir.path(), LINK_FAVORITES_FILENAME).unwrap().is_empty());
        dir.mkdir(".ant");
        fs::write(file(dir.path(), LINK_FAVORITES_FILENAME), "  \n").unwrap();
        assert!(read(dir.path(), LINK_FAVORITES_FILENAME).unwrap().is_empty());
    }

    #[test]
    fn adding_creates_the_file_and_removing_drops_the_name() {
        let dir = TempDir::new("link-list-set");
        set(dir.path(), LINK_FAVORITES_FILENAME, "204.Video-edition", true).unwrap();
        set(dir.path(), LINK_FAVORITES_FILENAME, "103.Toki", true).unwrap();
        assert_eq!(read(dir.path(), LINK_FAVORITES_FILENAME).unwrap(), ["103.Toki", "204.Video-edition"]);
        set(dir.path(), LINK_FAVORITES_FILENAME, "103.Toki", false).unwrap();
        assert_eq!(read(dir.path(), LINK_FAVORITES_FILENAME).unwrap(), ["204.Video-edition"]);
        set(dir.path(), LINK_FAVORITES_FILENAME, "204.Video-edition", false).unwrap();
        assert_eq!(fs::read_to_string(file(dir.path(), LINK_FAVORITES_FILENAME)).unwrap(), "[]\n");
    }

    #[test]
    fn each_list_keeps_its_own_names() {
        let dir = TempDir::new("link-list-separate");
        set(dir.path(), LINK_HIDDEN_FILENAME, "100.A", true).unwrap();
        assert!(read(dir.path(), LINK_FAVORITES_FILENAME).unwrap().is_empty());
        assert_eq!(read(dir.path(), LINK_HIDDEN_FILENAME).unwrap(), ["100.A"]);
    }

    #[test]
    fn a_file_that_is_not_a_list_is_reported() {
        let dir = TempDir::new("link-list-bad");
        dir.mkdir(".ant");
        fs::write(file(dir.path(), LINK_FAVORITES_FILENAME), "{ not: a list").unwrap();
        assert!(read(dir.path(), LINK_FAVORITES_FILENAME).is_err());
    }
}
