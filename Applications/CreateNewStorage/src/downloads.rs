// FolderManager download: helpful but not essential, so failures only warn.
use std::path::Path;

use crate::config::{FOLDER_MANAGER_ASSET, FOLDER_MANAGER_NAME, LATEST_JSON_URL, RELEASES_PAGE_URL};
use crate::fetch::{download, download_text, MAX_EXE_BYTES};
use crate::fs_ops::Backups;
#[cfg(not(windows))]
use crate::fs_ops::write_file_atomic;
use crate::sha256::sha256_hex;
use crate::Level;

pub fn optional_downloads(root: &Path, folder_manager: bool, backups: &mut Backups, progress: &mut dyn FnMut(Level, &str)) {
    if folder_manager {
        progress(Level::Step, "Downloading FolderManager ...");
        download_folder_manager(root, backups, progress);
    }
}

fn not_installed(progress: &mut dyn FnMut(Level, &str), level: Level, why: &str) {
    let mark = if level == Level::Error { "✖" } else { "⚠" };
    progress(level, &format!("{mark} FolderManager was not installed: {why}"));
    progress(Level::Warn, &format!("⚠ The storage was created without it. Download {FOLDER_MANAGER_ASSET} from {RELEASES_PAGE_URL} and put it in the storage folder."));
}

// The portable asset of the newest build goes into the storage root, only after its SHA-256 matches latest.json.
fn download_folder_manager(root: &Path, backups: &mut Backups, progress: &mut dyn FnMut(Level, &str)) {
    let latest = match download_text(LATEST_JSON_URL) {
        Ok(text) => text,
        Err(e) => return not_installed(progress, Level::Warn, &format!("latest.json could not be read ({e})")),
    };
    let bytes = match fetch_verified(&latest, FOLDER_MANAGER_ASSET, &mut |url| download(url, MAX_EXE_BYTES, 300)) {
        Ok(bytes) => bytes,
        Err(e) => return not_installed(progress, Level::Error, &e),
    };
    let exe = root.join(FOLDER_MANAGER_NAME);
    match install_folder_manager(&exe, &bytes, backups).and_then(|()| make_executable(&exe)) {
        Ok(()) => progress(Level::Ok, &format!("✔ {} (downloaded, SHA-256 verified)", exe.display())),
        Err(e) => not_installed(progress, Level::Error, &e),
    }
}

// Value text right after `"key":`, or None when the key is absent.
fn value_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let at = text.find(&format!("\"{key}\""))? + key.len() + 2;
    text[at..].trim_start().strip_prefix(':').map(str::trim_start)
}

fn string_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let value = value_after(text, key)?.strip_prefix('"')?.split('"').next()?;
    (!value.contains('\\')).then_some(value)
}

// (sha256, url) of one asset; a schema other than 1 or any missing field means no install.
fn asset_entry<'a>(latest: &'a str, asset: &str) -> Result<(&'a str, &'a str), String> {
    let schema = value_after(latest, "schema").map(|v| v.chars().take_while(char::is_ascii_digit).collect::<String>());
    if schema.as_deref() != Some("1") {
        return Err("latest.json has an unsupported schema".to_string());
    }
    let listed = &latest[latest.find("\"assets\"").ok_or("latest.json lists no assets")?..];
    let from = listed.find(&format!("\"{asset}\"")).ok_or_else(|| format!("latest.json has no {asset}"))?;
    let entry = &listed[from..];
    let entry = &entry[..entry.find('}').unwrap_or(entry.len())];
    let sha = string_after(entry, "sha256").filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()));
    let url = string_after(entry, "url").filter(|u| u.starts_with("https://github.com/"));
    sha.zip(url).ok_or_else(|| format!("latest.json has an incomplete entry for {asset}"))
}

// The bytes come back only when their SHA-256 equals the one latest.json publishes for the asset.
fn fetch_verified(latest: &str, asset: &str, get: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>) -> Result<Vec<u8>, String> {
    let (sha, url) = asset_entry(latest, asset)?;
    let bytes = get(url).map_err(|e| format!("{asset} could not be downloaded ({e})"))?;
    let actual = sha256_hex(&bytes);
    match actual.eq_ignore_ascii_case(sha) {
        true => Ok(bytes),
        false => Err(format!("the SHA-256 of {asset} ({actual}) does not match latest.json ({sha}); the file was discarded")),
    }
}

#[cfg(windows)]
fn install_folder_manager(exe: &Path, asset: &[u8], backups: &mut Backups) -> Result<(), String> {
    backups.write(exe, asset)
}

#[cfg(not(windows))]
fn install_folder_manager(exe: &Path, asset: &[u8], backups: &mut Backups) -> Result<(), String> {
    let archive = exe.with_extension("tar.gz");
    write_file_atomic(&archive, asset)?;
    let unpacked = std::process::Command::new("tar").arg("-xzOf").arg(&archive).output();
    let removed = std::fs::remove_file(&archive);
    let unpacked = unpacked.map_err(|e| format!("Could not run tar: {e}"))?;
    if !unpacked.status.success() {
        return Err(format!("Could not unpack {FOLDER_MANAGER_ASSET}: {}", String::from_utf8_lossy(&unpacked.stderr).trim()));
    }
    removed.map_err(|e| format!("Could not remove {}: {e}", archive.display()))?;
    backups.write(exe, &unpacked.stdout)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("Could not make {} executable: {e}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const ASSET: &str = "FolderManager_portable.exe";

    fn latest(schema: u32, sha: &str) -> String {
        format!(r#"{{"schema":{schema},"version":"2.4.5","extra":{{"x":1}},"assets":{{"{ASSET}":{{"sha256":"{sha}","url":"https://github.com/o/r/releases/download/t/{ASSET}"}},"SHA256SUMS":{{"sha256":"{sha}","url":"https://github.com/o/r/x"}}}}}}"#)
    }

    #[test]
    fn a_matching_hash_returns_the_bytes() {
        let mut asked = Vec::new();
        let got = fetch_verified(&latest(1, ABC), ASSET, &mut |u| {
            asked.push(u.to_string());
            Ok(b"abc".to_vec())
        });
        assert_eq!(got.unwrap(), b"abc");
        assert_eq!(asked, [format!("https://github.com/o/r/releases/download/t/{ASSET}")]);
    }

    #[test]
    fn a_hash_mismatch_discards_the_download() {
        let err = fetch_verified(&latest(1, ABC), ASSET, &mut |_| Ok(b"abd".to_vec())).unwrap_err();
        assert!(err.contains("does not match"), "{err}");
    }

    #[test]
    fn other_schemas_missing_assets_and_bad_entries_are_refused_without_downloading() {
        let never = &mut |_: &str| -> Result<Vec<u8>, String> { panic!("must not download") };
        assert!(fetch_verified(&latest(2, ABC), ASSET, never).unwrap_err().contains("schema"));
        assert!(fetch_verified(&latest(1, ABC), "Other.exe", never).unwrap_err().contains("no Other.exe"));
        assert!(fetch_verified(&latest(1, "abc"), ASSET, never).unwrap_err().contains("incomplete"));
        let off_host = latest(1, ABC).replace("https://github.com/o/r/releases", "https://evil.com/releases");
        assert!(fetch_verified(&off_host, ASSET, never).unwrap_err().contains("incomplete"));
    }

    #[test]
    fn a_failed_download_is_reported() {
        let err = fetch_verified(&latest(1, ABC), ASSET, &mut |_| Err("offline".to_string())).unwrap_err();
        assert!(err.contains("offline"), "{err}");
    }
}
