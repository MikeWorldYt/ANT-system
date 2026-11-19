// Live progress of the copy the window waits on: copies run in chunks so they can report, pause and be cancelled.
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;

const CHUNK: usize = 1 << 20;
const PAUSE_POLL: Duration = Duration::from_millis(100);

#[derive(Clone, Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: u64,
    pub action: String,
    pub from: String,
    pub to: String,
    pub bytes_total: u64,
    pub bytes_done: u64,
    pub files_total: u64,
    pub files_done: u64,
    pub current: String,
    pub paused: bool,
    pub finished: bool,
}

static JOB: Mutex<Option<Progress>> = Mutex::new(None);
static NEXT: Mutex<u64> = Mutex::new(0);
static CANCEL: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);

fn with(f: impl FnOnce(&mut Progress)) {
    if let Some(job) = JOB.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        f(job);
    }
}

pub fn snapshot() -> Option<Progress> {
    JOB.lock().unwrap_or_else(|e| e.into_inner()).clone().map(|mut p| {
        p.paused = PAUSED.load(Ordering::Relaxed);
        p
    })
}

pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
}

pub fn set_paused(on: bool) {
    PAUSED.store(on, Ordering::Relaxed);
}

// Links are not followed, as the copies themselves skip or refuse them.
fn measure(path: &Path, bytes: &mut u64, files: &mut u64) {
    let Ok(meta) = fs::symlink_metadata(path) else { return };
    if meta.is_file() {
        *bytes += meta.len();
        *files += 1;
    } else if meta.is_dir() && !meta.file_type().is_symlink() {
        for entry in fs::read_dir(path).into_iter().flatten().flatten() {
            measure(&entry.path(), bytes, files);
        }
    }
}

pub struct Job;

impl Drop for Job {
    fn drop(&mut self) {
        *JOB.lock().unwrap_or_else(|e| e.into_inner()) = None;
        PAUSED.store(false, Ordering::Relaxed);
        CANCEL.store(false, Ordering::Relaxed);
    }
}

pub fn size_of(path: &Path) -> u64 {
    let (mut bytes, mut files) = (0, 0);
    measure(path, &mut bytes, &mut files);
    bytes
}

// A copy counts every file it carries; a deletion counts the items it was given, since each goes to the bin whole.
pub fn begin(sources: &[PathBuf], to: &str) -> Job {
    begin_as("Copying", sources, to, false)
}

pub fn begin_as(action: &str, sources: &[PathBuf], to: &str, per_item: bool) -> Job {
    let (mut bytes_total, mut files_total) = (0, 0);
    for s in sources {
        measure(s, &mut bytes_total, &mut files_total);
    }
    if per_item {
        files_total = sources.len() as u64;
    }
    let id = {
        let mut next = NEXT.lock().unwrap_or_else(|e| e.into_inner());
        *next += 1;
        *next
    };
    let from = sources.first().and_then(|p| p.parent()).and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    CANCEL.store(false, Ordering::Relaxed);
    PAUSED.store(false, Ordering::Relaxed);
    *JOB.lock().unwrap_or_else(|e| e.into_inner()) = Some(Progress { id, action: action.to_string(), from, to: to.to_string(), bytes_total, files_total, ..Progress::default() });
    Job
}

pub fn item_done(name: &str, bytes: u64) {
    with(|j| {
        j.current = name.to_string();
        j.bytes_done += bytes;
        j.files_done += 1;
    });
}

// Called between items, so a paused deletion waits there and a cancelled one stops before the next item.
pub fn checkpoint() -> io::Result<()> {
    wait_or_cancel()
}

fn wait_or_cancel() -> io::Result<()> {
    while PAUSED.load(Ordering::Relaxed) && !CANCEL.load(Ordering::Relaxed) {
        std::thread::sleep(PAUSE_POLL);
    }
    if CANCEL.load(Ordering::Relaxed) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled by the user"));
    }
    Ok(())
}

// Like fs::copy, it keeps the modified time, so the copy reads as the same file; the caller removes a partial copy on error.
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    with(|j| j.current = from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    let mut src = File::open(from)?;
    let meta = src.metadata()?;
    let mut dst = fs::OpenOptions::new().write(true).create(true).truncate(true).open(to)?;
    let mut buf = vec![0; CHUNK];
    let mut total = 0;
    loop {
        wait_or_cancel()?;
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])?;
        total += n as u64;
        with(|j| j.bytes_done += n as u64);
    }
    dst.set_permissions(meta.permissions())?;
    dst.set_modified(meta.modified()?)?;
    with(|j| j.files_done += 1);
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn copies_report_bytes_and_files_and_keep_the_modified_time() {
        let dir = TempDir::new("transfer");
        let a = dir.path().join("a.bin");
        fs::write(&a, vec![7u8; 3000]).unwrap();
        let job = begin(&[a.clone()], "Dest");
        assert_eq!(snapshot().map(|p| (p.bytes_total, p.files_total)), Some((3000, 1)));
        copy(&a, &dir.path().join("b.bin")).unwrap();
        // Other tests copy in parallel through the same meter, so only a floor is certain.
        let p = snapshot().unwrap();
        assert!(p.bytes_done >= 3000 && p.files_done >= 1, "{p:?}");
        assert_eq!(fs::metadata(&a).unwrap().modified().unwrap(), fs::metadata(dir.path().join("b.bin")).unwrap().modified().unwrap());
        drop(job);
    }
}
