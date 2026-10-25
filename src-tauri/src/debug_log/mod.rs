// Opt-in Markdown trail of what the app did, written under .ant/debug for bug reports.
pub mod sysinfo;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use crate::config::{DATA_DIR, MAX_DEPTH};
use crate::error::{FmError, R};
use crate::storage::db::db_path;
use crate::updater::CURRENT_VERSION;
use sysinfo::SysInfo;

const MAX_MESSAGE_CHARS: usize = 2000;
const LEVELS: [&str; 3] = ["info", "warning", "error"];

static STATE: Mutex<Option<(PathBuf, Instant)>> = Mutex::new(None);

fn state() -> std::sync::MutexGuard<'static, Option<(PathBuf, Instant)>> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn io_err(path: &Path, e: &std::io::Error) -> FmError {
    FmError::Internal { message: format!("Could not write the debug log {}: {e}", path.display()) }
}

pub fn logs_dir(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join("debug")
}

pub fn valid_stamp(stamp: &str) -> bool {
    let groups: Vec<&str> = stamp.split('-').collect();
    groups.len() == 5 && groups.iter().all(|g| g.len() == 2 && g.bytes().all(|b| b.is_ascii_digit()))
}

fn free_path(dir: &Path, stamp: &str) -> PathBuf {
    let first = dir.join(format!("{stamp}_ANT-fm.log"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stamp}_ANT-fm_{n}.log"))).find(|p| !p.exists()).unwrap_or(first)
}

fn entry(seconds: f64, level: &str, message: &str) -> String {
    let level = if LEVELS.contains(&level) { level } else { "info" };
    let text: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    let text = text.replace("\r\n", "\n").replace(['\r', '\n'], " ⏎ ");
    format!("- +{seconds:.1}s [{level}] {text}\n")
}

fn header(root: &Path, info: &SysInfo) -> String {
    let build = if cfg!(debug_assertions) { "debug" } else { "release" };
    let webview = tauri::webview_version().unwrap_or_else(|_| "unknown".to_string());
    let db = fs::metadata(db_path(root)).map(|m| m.len().to_string()).unwrap_or_else(|_| "missing".to_string());
    let rows = [
        ("App", "ANT System Manager (FolderManager)".to_string()),
        ("Version", CURRENT_VERSION.to_string()),
        ("Build", build.to_string()),
        ("OS", info.os.clone()),
        ("Architecture", std::env::consts::ARCH.to_string()),
        ("RAM", info.ram.clone()),
        ("CPU", info.cpu.clone()),
        ("GPU", info.gpu.clone()),
        ("WebView", webview),
        ("Storage", root.display().to_string()),
        ("Tree depth limit", MAX_DEPTH.to_string()),
        ("ant.db", db),
        ("Process", format!("pid {}", std::process::id())),
    ];
    let table: String = rows.iter().map(|(k, v)| format!("| {k} | {v} |\n")).collect();
    format!("# ANT System Manager debug log\n\n| | |\n| --- | --- |\n{table}\n## Log\n\n")
}

pub fn start(stamp: &str, root: &Path, info: &SysInfo) -> R<PathBuf> {
    if !valid_stamp(stamp) {
        return Err(FmError::Internal { message: format!("Invalid debug log stamp {stamp}") });
    }
    let dir = logs_dir(root);
    fs::create_dir_all(&dir).map_err(|e| io_err(&dir, &e))?;
    let path = free_path(&dir, stamp);
    let text = header(root, info) + &entry(0.0, "info", "Debug logging started");
    let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| io_err(&path, &e))?;
    file.write_all(text.as_bytes()).map_err(|e| io_err(&path, &e))?;
    *state() = Some((path.clone(), Instant::now()));
    Ok(path)
}

pub fn line(level: &str, message: &str) -> R<()> {
    let guard = state();
    let Some((path, started)) = guard.as_ref() else { return Ok(()) };
    let text = entry(started.elapsed().as_secs_f64(), level, message);
    let mut file = OpenOptions::new().append(true).open(path).map_err(|e| io_err(path, &e))?;
    file.write_all(text.as_bytes()).map_err(|e| io_err(path, &e))
}

pub fn stop() -> R<()> {
    let result = line("info", "Debug logging stopped");
    *state() = None;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn only_five_two_digit_groups_make_a_stamp() {
        assert!(valid_stamp("26-10-24-09-05"));
        assert!(!valid_stamp("26-10-24-09"));
        assert!(!valid_stamp("26-10-24-09-5"));
        assert!(!valid_stamp("26-10-24-09-0a"));
        assert!(!valid_stamp("../x-10-24-09-05"));
        assert!(!valid_stamp("26-10-24-09-05-00"));
    }

    #[test]
    fn an_existing_log_name_gets_a_numeric_suffix() {
        let dir = TempDir::new("debug-name");
        let first = free_path(dir.path(), "26-10-24-09-05");
        assert!(first.ends_with("26-10-24-09-05_ANT-fm.log"));
        fs::write(&first, "").unwrap();
        let second = free_path(dir.path(), "26-10-24-09-05");
        assert!(second.ends_with("26-10-24-09-05_ANT-fm_2.log"));
        fs::write(&second, "").unwrap();
        assert!(free_path(dir.path(), "26-10-24-09-05").ends_with("26-10-24-09-05_ANT-fm_3.log"));
    }

    #[test]
    fn entries_flatten_newlines_cap_length_and_fall_back_to_info() {
        assert_eq!(entry(12.34, "warning", "a\r\nb\nc"), "- +12.3s [warning] a ⏎ b ⏎ c\n");
        assert_eq!(entry(0.0, "debug", "x"), "- +0.0s [info] x\n");
        let long = entry(1.0, "error", &"é".repeat(3000));
        assert_eq!(long.chars().filter(|&c| c == 'é').count(), MAX_MESSAGE_CHARS);
    }
}
