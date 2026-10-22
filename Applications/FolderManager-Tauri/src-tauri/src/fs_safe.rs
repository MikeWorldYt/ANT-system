// Guards every disk change: name validation, paths kept inside the storage, atomic writes and an undo journal.
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{FmError, R};

const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
const MAX_NAME_BYTES: usize = 255;
const RESERVED_DEVICES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn validate_component(name: &str) -> R<()> {
    let reject = |reason: &str| FmError::InvalidName {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    if name.is_empty() {
        return Err(reject("empty name"));
    }
    if name == "." || name == ".." {
        return Err(reject("dot segment"));
    }
    if name.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(reject("forbidden character"));
    }
    if name.chars().any(char::is_control) {
        return Err(reject("control character"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(reject("trailing dot or space"));
    }
    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED_DEVICES.iter().any(|d| d.eq_ignore_ascii_case(stem)) {
        return Err(reject("reserved Windows device name"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(reject("longer than 255 bytes"));
    }
    Ok(())
}

pub fn io_err(op: &str, path: &std::path::Path, e: &std::io::Error) -> FmError {
    FmError::Io {
        op: op.to_string(),
        path: path.display().to_string(),
        message: e.to_string(),
        os_code: e.raw_os_error(),
    }
}

pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> R<()> {
    use std::io::Write;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    let written = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        // The write error is what gets reported; a leftover tmp file is only clutter.
        let _ = std::fs::remove_file(&tmp);
        return Err(io_err("write file", path, &e));
    }
    Ok(())
}

pub fn resolve(root: &Path, rel: &[String]) -> R<PathBuf> {
    let invalid = |reason: String| FmError::InvalidPath { rel_parts: rel.to_vec(), reason };
    for part in rel {
        validate_component(part).map_err(|e| match e {
            FmError::InvalidName { name, reason } => invalid(format!("{name}: {reason}")),
            other => other,
        })?;
    }
    let canonical_root = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?;
    let mut existing = root.to_path_buf();
    for part in rel {
        let next = existing.join(part);
        match fs::symlink_metadata(&next) {
            Ok(meta) if is_link(&meta) => {
                return Err(invalid(format!("{part} is a link or reparse point")));
            }
            Ok(_) => existing = next,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(io_err("inspect path", &next, &e)),
        }
    }
    let canonical = fs::canonicalize(&existing).map_err(|e| io_err("resolve path", &existing, &e))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(invalid("outside the storage".to_string()));
    }
    Ok(join_rel(root, rel))
}

pub(crate) fn join_rel(root: &Path, rel: &[String]) -> PathBuf {
    rel.iter().fold(root.to_path_buf(), |p, part| p.join(part))
}

// Junctions and cloud placeholders are reparse points without being symlinks to std.
pub(crate) fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_type().is_symlink() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

// Windows compares names case-insensitively, so this decides what counts as the same folder.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

pub(crate) fn error_text(e: &FmError) -> String {
    match e {
        FmError::Io { op, path, message, .. } => format!("{op} {path}: {message}"),
        other => format!("{other:?}"),
    }
}

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
                errors.push(error_text(&e));
            }
        }
        errors
    }

    pub fn commit(self) {}

    pub fn abort(self, op: &str, cause: FmError) -> FmError {
        if self.steps.is_empty() {
            return cause;
        }
        let cause = error_text(&cause);
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

// Paths travel as environment variables and stdin, never inside the script text.
#[cfg(windows)]
pub(crate) fn run_powershell(
    script: &str,
    env: &[(&str, &std::ffi::OsStr)],
    stdin: Option<String>,
) -> Result<String, String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // An absolute path keeps a powershell.exe planted next to the app or on PATH from running.
    let exe = std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join(r"System32\WindowsPowerShell\v1.0\powershell.exe"))
        .unwrap_or_else(|| PathBuf::from("powershell.exe"));
    let mut child = Command::new(exe)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .envs(env.iter().copied())
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Could not start PowerShell: {e}"))?;
    // Writing on another thread avoids a deadlock when PowerShell fills stdout before reading all input.
    let writer = match (stdin, child.stdin.take()) {
        (Some(text), Some(mut pipe)) => Some(std::thread::spawn(move || pipe.write_all(text.as_bytes()))),
        _ => None,
    };
    let output = child.wait_with_output().map_err(|e| format!("PowerShell did not finish: {e}"))?;
    if let Some(handle) = writer {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(format!("Could not send input to PowerShell: {e}")),
            Err(_) => return Err("PowerShell input writer panicked".to_string()),
        }
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        return Ok(stdout);
    }
    let detail = if stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).trim().to_string()
    } else {
        stdout
    };
    Err(format!("PowerShell failed ({}): {detail}", output.status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml_store::test_support::TempDir;

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_joins_valid_parts_even_when_missing() {
        let dir = TempDir::new("resolve-ok");
        dir.mkdir("01.A");
        let got = resolve(dir.path(), &rel(&["01.A", "02.Missing"])).unwrap();
        assert_eq!(got, dir.path().join("01.A").join("02.Missing"));
        assert_eq!(resolve(dir.path(), &[]).unwrap(), dir.path());
    }

    #[test]
    fn resolve_refuses_traversal_and_absolute_parts() {
        let dir = TempDir::new("resolve-bad");
        let cases: [&[&str]; 5] = [&["..", "x"], &["01.A", ".."], &["a/b"], &["C:\\Windows"], &["\\\\server"]];
        for bad in cases {
            let err = resolve(dir.path(), &rel(bad)).unwrap_err();
            assert!(matches!(err, FmError::InvalidPath { .. }), "{bad:?} gave {err:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn resolve_refuses_junctions_and_symlinks() {
        let dir = TempDir::new("resolve-link");
        let outside = TempDir::new("resolve-outside");
        let link = dir.path().join("01.Link");
        let made = std::os::windows::fs::symlink_dir(outside.path(), &link).is_ok()
            || std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&link)
                .arg(outside.path())
                .output()
                .is_ok_and(|o| o.status.success());
        if !made {
            eprintln!("skipped: could not create a symlink or junction");
            return;
        }
        let err = resolve(dir.path(), &rel(&["01.Link", "x"])).unwrap_err();
        assert!(matches!(err, FmError::InvalidPath { .. }), "{err:?}");
        fs::remove_dir(&link).unwrap();
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
    fn accepts_ordinary_names() {
        assert!(validate_component("01.Math").is_ok());
        assert!(validate_component("a.b").is_ok());
        assert!(validate_component("COM10").is_ok());
        assert!(validate_component("CONSOLE").is_ok());
        assert!(validate_component("LPT0").is_ok());
        assert!(validate_component(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn rejects_empty_and_dot_segments() {
        assert!(validate_component("").is_err());
        assert!(validate_component(".").is_err());
        assert!(validate_component("..").is_err());
    }

    #[test]
    fn rejects_forbidden_and_control_characters() {
        for name in ["a/b", "a\\b", "a:b", "a*", "a?", "a\"", "a<", "a>", "a|", "a\0b", "a\tb"] {
            assert!(validate_component(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        assert!(validate_component("name.").is_err());
        assert!(validate_component("name ").is_err());
    }

    #[test]
    fn rejects_device_names_in_any_case_and_with_extension() {
        for name in ["CON", "con", "Nul.txt", "COM1", "lpt9.log", "aux.tar.gz"] {
            assert!(validate_component(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn rejects_names_longer_than_255_bytes() {
        assert!(validate_component(&"a".repeat(256)).is_err());
        assert!(validate_component(&"é".repeat(128)).is_err());
    }
}
