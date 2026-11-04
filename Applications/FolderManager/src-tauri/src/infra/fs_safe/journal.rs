// Records each rename and mkdir of a mutation so a failure can put the disk back as it was.
use std::fs;
use std::path::{Path, PathBuf};

use super::{io_err, same_name};
use crate::error::{FmError, R};

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
        ensure_target_free(from, to)?;
        fs::rename(from, to).map_err(|e| io_err("rename", from, &e))?;
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
                Step::Renamed { from, to } => ensure_target_free(to, from)
                    .and_then(|_| fs::rename(to, from).map_err(|e| io_err("undo rename", to, &e))),
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

// fs::rename may replace an existing target, so occupied names are refused unless only the case changes.
fn ensure_target_free(from: &Path, to: &Path) -> R<()> {
    match fs::symlink_metadata(to) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err("inspect path", to, &e)),
        Ok(_) if is_case_only_change(from, to) => Ok(()),
        Ok(_) => Err(FmError::AlreadyExists {
            name: to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        }),
    }
}

fn is_case_only_change(from: &Path, to: &Path) -> bool {
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    from.parent() == to.parent()
        && matches!((name(from), name(to)), (Some(a), Some(b)) if same_name(&a, &b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

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
