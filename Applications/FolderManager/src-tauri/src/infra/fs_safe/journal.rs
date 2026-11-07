// Records each rename and mkdir of a mutation so a failure can put the disk back as it was.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::io_err;
#[cfg(not(windows))]
use super::same_name;
use crate::domain::error::{FmError, R};

enum Step {
    Renamed { from: PathBuf, to: PathBuf },
    Created(PathBuf),
}

#[derive(Default)]
pub struct Journal {
    steps: Vec<Step>,
}

impl Journal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rename(&mut self, from: &Path, to: &Path) -> R<()> {
        rename_no_replace("rename", from, to)?;
        self.steps.push(Step::Renamed { from: from.to_path_buf(), to: to.to_path_buf() });
        Ok(())
    }

    pub fn mkdir(&mut self, path: &Path) -> R<()> {
        fs::create_dir(path).map_err(|e| io_err("create folder", path, &e))?;
        self.steps.push(Step::Created(path.to_path_buf()));
        Ok(())
    }

    // Created folders are removed non-recursively so an undo can never take user files with it.
    pub fn rollback(self) -> Vec<String> {
        let mut errors = Vec::new();
        for step in self.steps.into_iter().rev() {
            let undone = match &step {
                Step::Renamed { from, to } => rename_no_replace("undo rename", to, from),
                Step::Created(path) => {
                    fs::remove_dir(path).map_err(|e| io_err("undo create folder", path, &e))
                }
            };
            if let Err(e) = undone {
                errors.push(e.to_string());
            }
        }
        errors
    }

    pub fn commit(self) {}

    pub fn abort(self, op: &str, cause: FmError) -> FmError {
        if self.steps.is_empty() {
            return cause;
        }
        let cause = cause.to_string();
        let rollback_errors = self.rollback();
        if rollback_errors.is_empty() {
            FmError::RolledBack { op: op.to_string(), cause }
        } else {
            FmError::Inconsistent { op: op.to_string(), cause, rollback_errors }
        }
    }
}

// fs::rename replaces an occupied target, so the refusal happens inside the rename and no check-then-act gap is left; only a case change lands on its own name.
pub(crate) fn rename_no_replace(op: &str, from: &Path, to: &Path) -> R<()> {
    move_no_replace(from, to).map_err(|e| match e.kind() {
        io::ErrorKind::AlreadyExists => FmError::AlreadyExists {
            name: to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        },
        _ => io_err(op, from, &e),
    })
}

#[cfg(windows)]
fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let (from, to) = (wide_path(from)?, wide_path(to)?);
    // Without MOVEFILE_COPY_ALLOWED another drive still fails with the code a drop falls back on.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

// std adds the \\?\ prefix for itself past MAX_PATH; here a canonical parent gives it, and the last part stays as it is so a link is never followed.
#[cfg(windows)]
fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::ErrorKind::InvalidInput.into());
    };
    Ok(fs::canonicalize(parent)?.join(name).as_os_str().encode_wide().chain(Some(0)).collect())
}

#[cfg(not(windows))]
fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    match fs::symlink_metadata(to) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        Ok(_) if !is_case_only_change(from, to) => return Err(io::ErrorKind::AlreadyExists.into()),
        _ => {}
    }
    fs::rename(from, to)
}

#[cfg(not(windows))]
fn is_case_only_change(from: &Path, to: &Path) -> bool {
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    from.parent() == to.parent()
        && matches!((name(from), name(to)), (Some(a), Some(b)) if same_name(&a, &b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, TempDir};

    #[test]
    fn journal_rollback_undoes_renames_and_created_folders() {
        let dir = TempDir::new("journal");
        dir.mkdir("a");
        let p = |n: &str| dir.path().join(n);
        let mut journal = Journal::new();
        journal.rename(&p("a"), &p("b")).unwrap();
        journal.mkdir(&p("c")).unwrap();
        assert!(p("b").is_dir() && p("c").is_dir() && !p("a").exists());
        assert!(journal.rollback().is_empty());
        assert!(p("a").is_dir() && !p("b").exists() && !p("c").exists());
    }

    #[test]
    fn journal_refuses_occupied_targets_but_allows_case_only_renames() {
        let dir = TempDir::new("journal-target");
        dir.mkdir("a");
        dir.mkdir("b");
        let p = |n: &str| dir.path().join(n);
        let mut journal = Journal::new();
        assert!(matches!(journal.rename(&p("a"), &p("b")), Err(FmError::AlreadyExists { .. })));
        assert!(p("a").is_dir() && p("b").is_dir());
        journal.rename(&p("a"), &p("A")).unwrap();
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"A".to_string()), "{names:?}");
    }

    #[test]
    fn rename_no_replace_refuses_an_occupied_target_and_keeps_both_files() {
        let dir = TempDir::new("rename-occupied");
        let p = |n: &str| dir.path().join(n);
        fs::write(p("from.txt"), "from").unwrap();
        fs::write(p("to.txt"), "to").unwrap();
        let err = rename_no_replace("rename", &p("from.txt"), &p("to.txt")).unwrap_err();
        assert!(matches!(&err, FmError::AlreadyExists { name } if name == "to.txt"), "{err:?}");
        assert_eq!(fs::read_to_string(p("from.txt")).unwrap(), "from");
        assert_eq!(fs::read_to_string(p("to.txt")).unwrap(), "to");
    }

    #[test]
    fn rename_no_replace_refuses_an_occupied_folder() {
        let dir = TempDir::new("rename-occupied-dir");
        dir.mkdir("from");
        dir.mkdir("to/kept");
        let p = |n: &str| dir.path().join(n);
        let err = rename_no_replace("rename", &p("from"), &p("to")).unwrap_err();
        assert!(matches!(err, FmError::AlreadyExists { .. }), "{err:?}");
        assert!(p("from").is_dir() && p("to/kept").is_dir());
    }

    #[test]
    fn rename_no_replace_changes_only_the_case() {
        let dir = TempDir::new("rename-case");
        fs::write(dir.path().join("notes.txt"), "keep").unwrap();
        rename_no_replace("rename", &dir.path().join("notes.txt"), &dir.path().join("NOTES.txt")).unwrap();
        assert_eq!(crate::test_support::names(dir.path()), ["NOTES.txt"]);
        assert_eq!(fs::read_to_string(dir.path().join("NOTES.txt")).unwrap(), "keep");
    }

    #[test]
    fn rename_no_replace_works_past_the_old_path_limit() {
        let dir = TempDir::new("rename-long");
        let deep = dir.path().join("a".repeat(100)).join("b".repeat(100)).join("c".repeat(100));
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("from.txt"), "from").unwrap();
        rename_no_replace("rename", &deep.join("from.txt"), &deep.join("to.txt")).unwrap();
        assert!(deep.join("to.txt").is_file() && !deep.join("from.txt").exists());
    }

    #[test]
    fn journal_rollback_keeps_non_empty_created_folders() {
        let dir = TempDir::new("journal-keep");
        let made = dir.path().join("new");
        let mut journal = Journal::new();
        journal.mkdir(&made).unwrap();
        fs::write(made.join("user.txt"), "data").unwrap();
        assert_eq!(journal.rollback().len(), 1);
        assert!(made.join("user.txt").is_file());
    }

    #[test]
    fn journal_abort_without_steps_returns_the_cause() {
        let err = Journal::new().abort("op", FmError::NotMovable);
        assert!(matches!(err, FmError::NotMovable));
    }

    #[test]
    fn rolled_back_cause_is_readable_text() {
        let dir = TempDir::new("journal-cause");
        dir.mkdir("a");
        let mut journal = Journal::new();
        journal.rename(&dir.path().join("a"), &dir.path().join("b")).unwrap();
        let err = journal.abort("move", FmError::NotFound { rel_parts: rel(&["x", "y"]) });
        assert!(matches!(&err, FmError::RolledBack { cause, .. } if cause == "x/y was not found"), "{err:?}");
        assert!(dir.path().join("a").is_dir());
    }
}
