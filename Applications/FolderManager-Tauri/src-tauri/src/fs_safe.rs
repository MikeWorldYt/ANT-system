// Guards every disk change: name validation, paths kept inside the storage, atomic writes and an undo journal.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::config::{is_hidden_dir, CORE_FOLDER, SETTINGS_FOLDER};
use crate::error::{FmError, R};

const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
// NTFS limits a name to 255 UTF-16 code units, not bytes.
pub(crate) const MAX_NAME_UNITS: usize = 255;
const LOCK_FILE: &str = ".FolderManager.lock";
const STALE_LOCK: Duration = Duration::from_secs(120);
static TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);
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
    if name.encode_utf16().count() > MAX_NAME_UNITS {
        return Err(reject("longer than 255 characters"));
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
    // A per-process, per-call name keeps two writers from sharing one temp file.
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
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
        // The staging folder holds deleted folders and is never a node, whatever its case.
        if is_hidden_dir(part) {
            return Err(invalid(format!("{part}: reserved folder")));
        }
    }
    let canonical_root = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?;
    let mut existing = root.to_path_buf();
    for part in rel {
        let next = existing.join(part);
        match fs::symlink_metadata(&next) {
            Ok(meta) if redirects(&next, &meta)? => {
                return Err(invalid(format!("{part} is a link or junction")));
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

// Cloud placeholders (OneDrive) are reparse points that stay in place; links and junctions lead elsewhere.
pub(crate) fn redirects(path: &Path, meta: &fs::Metadata) -> R<bool> {
    if !is_link(meta) {
        return Ok(false);
    }
    if meta.file_type().is_symlink() {
        return Ok(true);
    }
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { return Ok(true) };
    let real = fs::canonicalize(path).map_err(|e| io_err("resolve path", path, &e))?;
    let parent = fs::canonicalize(parent).map_err(|e| io_err("resolve path", parent, &e))?;
    Ok(!same_name(&real.to_string_lossy(), &parent.join(name).to_string_lossy()))
}

// Windows compares names case-insensitively, so this decides what counts as the same folder.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

pub(crate) fn error_text(e: &FmError) -> String {
    e.to_string()
}

// Serialises mutations across processes, which the in-process Mutex cannot see.
pub struct StorageLock {
    path: PathBuf,
}

impl StorageLock {
    pub fn acquire(root: &Path) -> R<Self> {
        let dir = root.join(SETTINGS_FOLDER).join(CORE_FOLDER);
        fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
        let path = dir.join(LOCK_FILE);
        let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err(busy_error(&path)),
            Err(e) => return Err(io_err("lock storage", &path, &e)),
        };
        let lock = Self { path };
        std::io::Write::write_all(&mut file, std::process::id().to_string().as_bytes())
            .map_err(|e| io_err("lock storage", &lock.path, &e))?;
        Ok(lock)
    }
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        // A lock file left behind is reported as stale by the next mutation, which tells the user what to do.
        let _ = fs::remove_file(&self.path);
    }
}

// An old lock may belong to a crashed run or to a slow one, so it is reported and never removed here.
fn busy_error(path: &Path) -> FmError {
    let age = fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
    let message = match age {
        Some(age) if age >= STALE_LOCK => format!(
            "the storage has been locked for {} minutes; if no other Folder Manager is open, delete this file and retry",
            age.as_secs() / 60
        ),
        _ => "another Folder Manager is changing this storage; retry in a moment".to_string(),
    };
    FmError::Io { op: "lock storage".to_string(), path: path.display().to_string(), message, os_code: None }
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
    timeout: Duration,
) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::Instant;
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
    type Reader = std::thread::JoinHandle<std::io::Result<Vec<u8>>>;
    fn drain<P: Read + Send + 'static>(pipe: Option<P>) -> Option<Reader> {
        pipe.map(|mut p| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                p.read_to_end(&mut bytes).map(|_| bytes)
            })
        })
    }
    let out_reader = drain(child.stdout.take());
    let err_reader = drain(child.stderr.take());
    // A shell dialog nobody answers would otherwise block this thread forever.
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let stopped = child.kill().and_then(|_| child.wait());
                return Err(match stopped {
                    Ok(_) => format!("PowerShell did not finish within {} s and was stopped", timeout.as_secs()),
                    Err(e) => format!("PowerShell did not finish within {} s and could not be stopped: {e}", timeout.as_secs()),
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(format!("PowerShell did not finish: {e}")),
        }
    };
    if let Some(handle) = writer {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(format!("Could not send input to PowerShell: {e}")),
            Err(_) => return Err("PowerShell input writer panicked".to_string()),
        }
    }
    let collect = |reader: Option<Reader>| match reader {
        Some(handle) => match handle.join() {
            Ok(Ok(bytes)) => Ok(String::from_utf8_lossy(&bytes).trim().to_string()),
            Ok(Err(e)) => Err(format!("Could not read PowerShell output: {e}")),
            Err(_) => Err("PowerShell output reader panicked".to_string()),
        },
        None => Ok(String::new()),
    };
    let stdout = collect(out_reader)?;
    let stderr = collect(err_reader)?;
    if status.success() {
        return Ok(stdout);
    }
    let detail = if stdout.is_empty() { stderr } else { stdout };
    Err(format!("PowerShell failed ({status}): {detail}"))
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
    fn name_limit_counts_utf16_units() {
        assert!(validate_component(&"a".repeat(256)).is_err());
        assert!(validate_component(&"é".repeat(255)).is_ok());
        assert!(validate_component(&"\u{1F600}".repeat(127)).is_ok());
        assert!(validate_component(&"\u{1F600}".repeat(128)).is_err());
    }

    #[test]
    fn resolve_refuses_the_trash_folder_in_any_case_and_position() {
        let dir = TempDir::new("resolve-trash");
        dir.mkdir(".ant-trash/x");
        for bad in [&[".ant-trash"][..], &[".ANT-Trash", "x"], &["01.A", ".ant-trash"]] {
            let err = resolve(dir.path(), &rel(bad)).unwrap_err();
            assert!(matches!(err, FmError::InvalidPath { .. }), "{bad:?} gave {err:?}");
        }
    }

    #[test]
    fn plain_folders_do_not_redirect() {
        let dir = TempDir::new("redirect-plain");
        dir.mkdir("a");
        let meta = fs::symlink_metadata(dir.path().join("a")).unwrap();
        assert!(!redirects(&dir.path().join("a"), &meta).unwrap());
    }

    #[cfg(windows)]
    #[test]
    fn junctions_redirect() {
        let dir = TempDir::new("redirect-junction");
        let outside = TempDir::new("redirect-outside");
        let link = dir.path().join("j");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .is_ok_and(|o| o.status.success());
        if !made {
            eprintln!("skipped: could not create a junction");
            return;
        }
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(redirects(&link, &meta).unwrap());
        fs::remove_dir(&link).unwrap();
    }

    #[test]
    fn write_atomic_uses_a_unique_temp_name_and_leaves_nothing_behind() {
        let dir = TempDir::new("atomic");
        let target = dir.path().join("Tree.yaml");
        fs::create_dir(dir.path().join(".Tree.yaml.tmp")).unwrap();
        write_atomic(&target, b"one").unwrap();
        write_atomic(&target, b"two").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "two");
        let mut left: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, [".Tree.yaml.tmp", "Tree.yaml"]);
    }

    #[test]
    fn storage_lock_is_exclusive_and_released_on_drop() {
        let dir = TempDir::new("lock");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let busy = StorageLock::acquire(dir.path()).err().unwrap();
        assert!(matches!(&busy, FmError::Io { message, .. } if message.contains("another Folder Manager")), "{busy:?}");
        drop(held);
        drop(StorageLock::acquire(dir.path()).unwrap());
        assert!(!dir.path().join(SETTINGS_FOLDER).join(CORE_FOLDER).join(LOCK_FILE).exists());
    }

    #[test]
    fn stale_storage_lock_is_reported_and_kept() {
        let dir = TempDir::new("lock-stale");
        let path = dir.path().join(SETTINGS_FOLDER).join(CORE_FOLDER).join(LOCK_FILE);
        dir.mkdir("999.Settings/00.Core");
        let file = fs::File::create(&path).unwrap();
        file.set_modified(std::time::SystemTime::now() - Duration::from_secs(600)).unwrap();
        drop(file);
        let err = StorageLock::acquire(dir.path()).err().unwrap();
        assert!(matches!(&err, FmError::Io { message, .. } if message.contains("delete this file")), "{err:?}");
        assert!(path.is_file());
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

    #[cfg(windows)]
    #[test]
    #[ignore = "starts PowerShell"]
    fn run_powershell_stops_a_child_that_outlives_the_timeout() {
        let started = std::time::Instant::now();
        let err = run_powershell("Start-Sleep -Seconds 60", &[], None, Duration::from_secs(2)).unwrap_err();
        assert!(err.contains("did not finish within 2 s"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(20));
    }
}
