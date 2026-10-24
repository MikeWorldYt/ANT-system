// Guards every disk change: name validation, paths kept inside the storage, atomic writes and an undo journal.
mod journal;
mod lock;
#[cfg(windows)]
mod powershell;
mod validate;

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::{FmError, R};

pub(crate) use journal::Journal;
pub(crate) use lock::{StorageLock, LOCK_FILE};
#[cfg(windows)]
pub(crate) use powershell::{run_powershell, CREATE_NO_WINDOW};
pub(crate) use validate::{
    is_link, join_rel, redirects, resolve, same_name, validate_component, FORBIDDEN, MAX_NAME_UNITS,
};

static TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::TempDir;

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
}
