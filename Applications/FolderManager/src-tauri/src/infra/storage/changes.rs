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
    use proptest::prelude::*;

    use super::*;
    use crate::test_support::{rel, TempDir};

    fn entry(seq: u64, writer: u32, level: &[&str], rescan: bool) -> Entry {
        Entry { seq, writer, levels: vec![rel(level)], rescan }
    }

    // Seqs run 1, 2, 3... as record() writes them; writer 1 is the window reading the log.
    fn gapless_log() -> impl Strategy<Value = Vec<Entry>> {
        let level = prop::collection::vec(prop::sample::select(vec!["100.WORK", "200.HOME", "02.B"]), 0..3);
        prop::collection::vec((1u32..3, level, any::<bool>()), 0..10).prop_map(|rows| {
            rows.into_iter().enumerate().map(|(i, (writer, level, rescan))| Entry { seq: i as u64 + 1, writer, levels: vec![level.into_iter().map(String::from).collect()], rescan }).collect()
        })
    }

    proptest! {
        #[test]
        fn a_gapless_log_brings_back_only_what_other_writers_did_since_the_last_read(log in gapless_log(), seen in 0u64..10) {
            prop_assume!(seen <= newest(&log));
            let others: Vec<&Entry> = log.iter().filter(|e| e.seq > seen && e.writer != 1).collect();
            let mut levels: Vec<Vec<String>> = others.iter().flat_map(|e| e.levels.clone()).collect();
            levels.sort();
            levels.dedup();
            let expected = (!others.is_empty()).then(|| News { levels, rescan: others.iter().any(|e| e.rescan) });
            prop_assert_eq!(news(&log, seen, 1), expected);
        }

        #[test]
        fn the_log_reads_back_as_it_was_written(log in gapless_log(), seq in any::<u64>(), writer in any::<u32>(), level in prop::collection::vec(".{0,8}", 0..4)) {
            let log: Vec<Entry> = log.into_iter().chain([Entry { seq, writer, levels: vec![level], rescan: false }]).collect();
            let bytes = serde_json::to_vec(&log).unwrap();
            prop_assert_eq!(serde_json::from_slice::<Vec<Entry>>(&bytes).unwrap(), log);
        }
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
