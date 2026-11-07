// A short log in the root .ant/ through which another process tells the open window which levels it changed, metadata included.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::core_dir::data_dir;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, write_atomic, StorageLock};

pub const CHANGES_FILE: &str = "changes.json";
// Covers a burst of tool calls inside one watcher batch; anything older that was missed becomes a rescan.
const KEEP: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub seq: u64,
    pub writer: u32,
    pub levels: Vec<Vec<String>>,
    pub rescan: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct News {
    pub levels: Vec<Vec<String>>,
    pub rescan: bool,
}

fn path(root: &Path) -> PathBuf {
    data_dir(root).join(CHANGES_FILE)
}

pub fn read(root: &Path) -> R<Vec<Entry>> {
    let path = path(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err("read change log", &path, &e)),
    };
    serde_json::from_str(&text).map_err(|e| FmError::Internal { message: format!("{} is unreadable: {e}", path.display()) })
}

// The storage lock keeps two writers from taking the same sequence number.
pub fn record(root: &Path, levels: &[Vec<String>], rescan: bool) -> R<()> {
    let _lock = StorageLock::acquire(root)?;
    // The log holds only signals, so an unreadable one starts over; the window reads the restart as a rescan.
    let mut entries = read(root).unwrap_or_else(|e| {
        eprintln!("Change log started over: {e}");
        Vec::new()
    });
    entries.push(Entry { seq: newest(&entries) + 1, writer: std::process::id(), levels: levels.to_vec(), rescan });
    entries.drain(..entries.len().saturating_sub(KEEP));
    let bytes = serde_json::to_vec(&entries).map_err(|e| FmError::Internal { message: e.to_string() })?;
    write_atomic(&path(root), &bytes)
}

pub fn newest(entries: &[Entry]) -> u64 {
    entries.last().map_or(0, |e| e.seq)
}

// Entries past `seen` written by other processes; a gap or a restarted log means some were missed, so the window rescans.
pub fn news(entries: &[Entry], seen: u64, me: u32) -> Option<News> {
    let newest = newest(entries);
    if newest == seen {
        return None;
    }
    let from = if newest < seen { 0 } else { seen };
    let fresh: Vec<&Entry> = entries.iter().filter(|e| e.seq > from).collect();
    let missed = newest < seen || fresh.first().is_some_and(|e| e.seq != from + 1);
    let others: Vec<&&Entry> = fresh.iter().filter(|e| e.writer != me).collect();
    if !missed && others.is_empty() {
        return None;
    }
    let mut levels: Vec<Vec<String>> = others.iter().flat_map(|e| e.levels.iter().cloned()).collect();
    levels.sort();
    levels.dedup();
    Some(News { levels, rescan: missed || others.iter().any(|e| e.rescan) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, TempDir};

    fn entry(seq: u64, writer: u32, level: &[&str], rescan: bool) -> Entry {
        Entry { seq, writer, levels: vec![rel(level)], rescan }
    }

    #[test]
    fn changes_from_other_writers_since_the_last_read_are_merged() {
        let log = [entry(1, 7, &["100.WORK"], false), entry(2, 8, &["200.HOME"], false), entry(3, 8, &["100.WORK"], false)];
        assert_eq!(news(&log, 3, 1), None);
        assert_eq!(news(&log, 1, 1), Some(News { levels: vec![rel(&["100.WORK"]), rel(&["200.HOME"])], rescan: false }));
        assert_eq!(news(&log, 2, 1), Some(News { levels: vec![rel(&["100.WORK"])], rescan: false }));
    }

    #[test]
    fn the_windows_own_entries_never_come_back_to_it() {
        let log = [entry(1, 7, &["100.WORK"], false), entry(2, 1, &["200.HOME"], true)];
        assert_eq!(news(&log, 1, 1), None);
        assert_eq!(news(&log, 0, 1), Some(News { levels: vec![rel(&["100.WORK"])], rescan: false }));
    }

    #[test]
    fn a_missed_entry_or_a_restarted_log_asks_for_a_rescan() {
        let log = [entry(5, 7, &["100.WORK"], false), entry(6, 7, &[], true)];
        assert!(news(&log, 2, 1).unwrap().rescan);
        assert!(news(&log[..1], 4, 1).is_some_and(|n| !n.rescan));
        assert!(news(&log, 9, 1).unwrap().rescan);
        assert!(news(&[], 9, 1).unwrap().rescan);
        assert!(news(&log[1..], 5, 1).unwrap().rescan);
    }

    #[test]
    fn recording_numbers_the_entries_and_keeps_only_the_latest() {
        let dir = TempDir::new("changes-log");
        assert_eq!(read(dir.path()).unwrap(), Vec::new());
        for n in 0..KEEP + 3 {
            record(dir.path(), &[rel(&["100.WORK", &n.to_string()])], false).unwrap();
        }
        let log = read(dir.path()).unwrap();
        assert_eq!((log.len(), log[0].seq, newest(&log)), (KEEP, 4, KEEP as u64 + 3));
        assert!(log.iter().all(|e| e.writer == std::process::id()));
        std::fs::write(path(dir.path()), "not json").unwrap();
        assert!(read(dir.path()).is_err());
        record(dir.path(), &[], true).unwrap();
        assert_eq!(read(dir.path()).unwrap(), vec![Entry { seq: 1, writer: std::process::id(), levels: vec![], rescan: true }]);
    }
}
