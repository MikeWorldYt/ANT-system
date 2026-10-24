// Moves a folder into the desktop trash on Unix without a new dependency, so it can still be restored from there.
use std::fs;
#[cfg(not(target_os = "macos"))]
use std::io::Write;
use std::path::{Path, PathBuf};

// EXDEV is 18 on both Linux and macOS.
const EXDEV: i32 = 18;

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).ok_or_else(|| "HOME is not set".to_string())
}

fn move_into(path: &Path, target: &Path) -> Result<(), String> {
    fs::rename(path, target).map_err(|e| {
        if e.raw_os_error() == Some(EXDEV) {
            "the folder is on another drive than the trash".to_string()
        } else {
            format!("Could not move {} to the trash: {e}", path.display())
        }
    })
}

fn file_name(path: &Path) -> Result<String, String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| format!("{} has no name", path.display()))
}

fn candidate(name: &str, n: u32) -> String {
    if n == 0 { name.to_string() } else { format!("{name}.{n}") }
}

#[cfg(target_os = "macos")]
pub(super) fn send(path: &Path) -> Result<(), String> {
    let trash = home()?.join(".Trash");
    let name = file_name(path)?;
    let target = (0..1000).map(|n| trash.join(candidate(&name, n))).find(|t| fs::symlink_metadata(t).is_err());
    move_into(path, &target.ok_or_else(|| "the trash has no free name for the folder".to_string())?)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn send(path: &Path) -> Result<(), String> {
    let data = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()).map(PathBuf::from);
    let trash = match data {
        Some(dir) => dir.join("Trash"),
        None => home()?.join(".local/share/Trash"),
    };
    let (files, info) = (trash.join("files"), trash.join("info"));
    for dir in [&files, &info] {
        fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    }
    let absolute = fs::canonicalize(path).map_err(|e| format!("Could not resolve {}: {e}", path.display()))?;
    let name = file_name(path)?;
    let body = format!("[Trash Info]\nPath={}\nDeletionDate={}\n", encode(&absolute), deletion_date());
    for n in 0..1000 {
        let entry = candidate(&name, n);
        let info_path = info.join(format!("{entry}.trashinfo"));
        let target = files.join(&entry);
        if fs::symlink_metadata(&target).is_ok() {
            continue;
        }
        // create_new claims the name, so two apps trashing at once never share one .trashinfo.
        let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&info_path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Could not write {}: {e}", info_path.display())),
        };
        let moved = file
            .write_all(body.as_bytes())
            .map_err(|e| format!("Could not write {}: {e}", info_path.display()))
            .and_then(|()| move_into(path, &target));
        if let Err(e) = moved {
            if let Err(cleanup) = fs::remove_file(&info_path) {
                eprintln!("Could not remove {}: {cleanup}", info_path.display());
            }
            return Err(e);
        }
        return Ok(());
    }
    Err("the trash has no free name for the folder".to_string())
}

#[cfg(not(target_os = "macos"))]
fn encode(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str()
        .as_bytes()
        .iter()
        .map(|&b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => char::from(b).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

// The spec asks for local time, which std cannot give; UTC only shifts the date file managers show.
#[cfg(not(target_os = "macos"))]
fn deletion_date() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Howard Hinnant's days-to-civil conversion.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}
