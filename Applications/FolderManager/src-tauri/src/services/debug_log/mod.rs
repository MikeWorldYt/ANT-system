// Opt-in Markdown trail of what the app did for bug reports, kept in the system temp folder so cleanups remove it.
pub mod sysinfo;

pub(crate) use crate::domain::validation::valid_stamp;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use crate::domain::error::{FmError, R};
use crate::infra::storage::db::db_path;
use crate::services::tree::depth;
use crate::services::updater::CURRENT_VERSION;
use sysinfo::SysInfo;

const MAX_MESSAGE_CHARS: usize = 2000;
const LEVELS: [&str; 3] = ["info", "warning", "error"];
const KEEP: Duration = Duration::from_secs(14 * 86_400);

static STATE: Mutex<Option<(PathBuf, Instant)>> = Mutex::new(None);

fn state() -> std::sync::MutexGuard<'static, Option<(PathBuf, Instant)>> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn io_err(path: &Path, e: &std::io::Error) -> FmError {
    FmError::Internal { message: format!("Could not write the debug log {}: {e}", path.display()) }
}

pub fn logs_dir() -> PathBuf {
    std::env::temp_dir().join("ANT").join("logs")
}

// One file per day: every session of that day appends to it instead of opening a new one.
fn day_path(dir: &Path, stamp: &str) -> PathBuf {
    dir.join(format!("{}_ANT-fm.log", &stamp[..8]))
}

fn prune(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let old = entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| SystemTime::now().duration_since(t).is_ok_and(|age| age > KEEP));
        if old && entry.file_name().to_string_lossy().ends_with(".log") {
            if let Err(e) = fs::remove_file(entry.path()) {
                eprintln!("Could not remove old log {}: {e}", entry.path().display());
            }
        }
    }
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
    let tree_depth = match depth::read(root) {
        Ok(setting) => setting.warning.map_or_else(|| setting.limit.to_string(), |w| format!("{} ({w})", setting.limit)),
        Err(e) => format!("unknown ({e})"),
    };
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
        ("Tree depth limit", tree_depth),
        ("ant.db", db),
        ("Process", format!("pid {}", std::process::id())),
    ];
    let table: String = rows.iter().map(|(k, v)| format!("| {k} | {v} |\n")).collect();
    format!("\n# ANT System Manager session\n\n| | |\n| --- | --- |\n{table}\n## Log\n\n")
}

pub fn start(stamp: &str, root: &Path, info: &SysInfo) -> R<PathBuf> {
    if !valid_stamp(stamp) {
        return Err(FmError::Internal { message: format!("Invalid debug log stamp {stamp}") });
    }
    let dir = logs_dir();
    fs::create_dir_all(&dir).map_err(|e| io_err(&dir, &e))?;
    prune(&dir);
    let path = day_path(&dir, stamp);
    let text = header(root, info) + &entry(0.0, "info", "Debug logging started");
    let mut file = OpenOptions::new().create(true).append(true).open(&path).map_err(|e| io_err(&path, &e))?;
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
    fn sessions_of_a_day_share_one_file() {
        let dir = TempDir::new("debug-name");
        assert!(day_path(dir.path(), "26-10-24-09-05").ends_with("26-10-24_ANT-fm.log"));
        assert_eq!(day_path(dir.path(), "26-10-24-09-05"), day_path(dir.path(), "26-10-24-23-59"));
    }

    #[test]
    fn entries_flatten_newlines_cap_length_and_fall_back_to_info() {
        assert_eq!(entry(12.34, "warning", "a\r\nb\nc"), "- +12.3s [warning] a ⏎ b ⏎ c\n");
        assert_eq!(entry(0.0, "debug", "x"), "- +0.0s [info] x\n");
        let long = entry(1.0, "error", &"é".repeat(3000));
        assert_eq!(long.chars().filter(|&c| c == 'é').count(), MAX_MESSAGE_CHARS);
    }
}
