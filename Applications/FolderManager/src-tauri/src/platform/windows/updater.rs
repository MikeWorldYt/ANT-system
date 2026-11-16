// Windows update path: the channel's latest.json, then the installer it names, checked against its SHA-256 and started by its own path.
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::domain::config::{INSTALLER_ASSET, LATEST_URL, RELEASE_API_URL, RELEASE_TAG_PREFIX};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::io_err;
use crate::platform::random::hex_token;
use crate::platform::windows::powershell::run_powershell;
use crate::services::updater::{newer_release, Release};

const FETCH: &str = "(Invoke-WebRequest -UseBasicParsing -Uri $env:FM_URL -TimeoutSec 5 -Headers @{'User-Agent'='FolderManager-Updater'}).Content";
// Windows PowerShell 5.1 downloads a large file many times slower while it draws the progress bar.
const DOWNLOAD: &str = "$ProgressPreference = 'SilentlyContinue'; Invoke-WebRequest -UseBasicParsing -Uri $env:FM_URL -OutFile $env:FM_OUT -TimeoutSec 60 -Headers @{'User-Agent'='FolderManager-Updater'}";

fn fail(detail: impl std::fmt::Display) -> FmError {
    FmError::Internal { message: format!("Could not update FolderManager: {detail}. FolderManager stays open") }
}

fn fetch(url: &str) -> Result<String, String> {
    run_powershell(FETCH, &[("FM_URL", url.as_ref())], None, Duration::from_secs(10))
}

pub fn latest_version() -> R<Option<String>> {
    Ok(newer_release(fetch(LATEST_URL).ok())?.map(|r| r.version))
}

// Nothing in the storage takes part: the installer is the asset latest.json names, run only when it matches the SHA-256 listed there.
pub fn launch_updater() -> R<()> {
    let json = fetch(LATEST_URL).map_err(|e| fail(format!("the update channel could not be read ({e})")))?;
    let release = newer_release(Some(json))?.ok_or_else(|| fail("the update channel offers no newer version"))?;
    check_api_digest(&release)?;
    let dir = std::env::temp_dir().join(format!("FolderManager-update-{}", hex_token(8).map_err(fail)?));
    std::fs::create_dir(&dir).map_err(|e| io_err("create the update folder", &dir, &e))?;
    let installer = dir.join(INSTALLER_ASSET);
    let checked = download(&release.installer_url, &installer).and_then(|()| verify(&installer, &release.sha256));
    if let Err(e) = checked {
        discard(&dir);
        return Err(e);
    }
    Command::new(&installer).current_dir(&dir).spawn().map(drop).map_err(|e| io_err("start the installer", &installer, &e))
}

// Extra defense: when the Releases API answers (it is rate limited), its digest must agree with latest.json.
fn check_api_digest(release: &Release) -> R<()> {
    let tag = format!("{RELEASE_TAG_PREFIX}v{}", release.version);
    let Ok(json) = fetch(&format!("{RELEASE_API_URL}{tag}")) else { return Ok(()) };
    match asset_sha256(&json, INSTALLER_ASSET) {
        Ok(api) if api != release.sha256 => Err(fail(format!("latest.json and the release {tag} list different SHA-256 values for {INSTALLER_ASSET}"))),
        _ => Ok(()),
    }
}

// GitHub computes the digest when the asset is uploaded; a release without one is refused.
fn asset_sha256(release: &str, name: &str) -> Result<String, String> {
    let json: Value = serde_json::from_str(release).map_err(|e| format!("the release data is not valid ({e})"))?;
    let asset = json["assets"].as_array().into_iter().flatten().find(|a| a["name"] == name).ok_or_else(|| format!("the release has no {name}"))?;
    let hex = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()));
    hex.map(str::to_ascii_lowercase).ok_or_else(|| format!("the release lists no SHA-256 for {name}"))
}

fn download(url: &str, to: &Path) -> R<()> {
    run_powershell(DOWNLOAD, &[("FM_URL", url.as_ref()), ("FM_OUT", to.as_os_str())], None, Duration::from_secs(600))
        .map(drop)
        .map_err(|e| fail(format!("the installer could not be downloaded ({e})")))
}

fn verify(file: &Path, expected: &str) -> R<()> {
    let bytes = std::fs::read(file).map_err(|e| io_err("read the installer", file, &e))?;
    let actual = sha256_hex(&bytes)?;
    if actual == expected {
        return Ok(());
    }
    Err(fail(format!("the installer does not match the SHA-256 of the release (expected {expected}, got {actual})")))
}

fn sha256_hex(data: &[u8]) -> R<String> {
    use windows_sys::Win32::Security::Cryptography::{BCryptHash, BCRYPT_SHA256_ALG_HANDLE};
    let len = u32::try_from(data.len()).map_err(|_| fail("the installer is too large"))?;
    let mut out = [0u8; 32];
    // SAFETY: the pointers and lengths describe `data` and `out`, and the SHA-256 pseudo-handle needs no opening.
    let status = unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, std::ptr::null(), 0, data.as_ptr(), len, out.as_mut_ptr(), 32) };
    if status != 0 {
        return Err(fail(format!("BCryptHash failed with status {status:#x}")));
    }
    Ok(out.iter().map(|b| format!("{b:02x}")).collect())
}

fn discard(dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        eprintln!("Could not remove {}: {e}", dir.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_matches_the_standard_vector() {
        assert_eq!(sha256_hex(b"abc").unwrap(), ABC);
    }

    #[test]
    fn the_installer_digest_is_read_from_the_release_and_a_missing_one_is_refused() {
        let release = format!(r#"{{"assets":[{{"name":"FolderManager_portable.exe","digest":"sha256:{}"}},{{"name":"{INSTALLER_ASSET}","digest":"sha256:{}"}}]}}"#, "0".repeat(64), ABC.to_uppercase());
        assert_eq!(asset_sha256(&release, INSTALLER_ASSET).unwrap(), ABC);
        let no_digest = format!(r#"{{"assets":[{{"name":"{INSTALLER_ASSET}","digest":null}}]}}"#);
        let other_hash = format!(r#"{{"assets":[{{"name":"{INSTALLER_ASSET}","digest":"sha1:{}"}}]}}"#, "0".repeat(40));
        for bad in [no_digest.as_str(), other_hash.as_str(), r#"{"assets":[]}"#, "not json"] {
            assert!(asset_sha256(bad, INSTALLER_ASSET).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_installer_that_does_not_match_the_release_is_refused() {
        let dir = TempDir::new("updater-hash");
        let file = dir.path().join(INSTALLER_ASSET);
        std::fs::write(&file, "abc").unwrap();
        assert!(verify(&file, ABC).is_ok());
        std::fs::write(&file, "abd").unwrap();
        let err = verify(&file, ABC).unwrap_err();
        assert!(err.to_string().contains("does not match"), "{err}");
    }
}
