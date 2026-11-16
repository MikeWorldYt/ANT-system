// Update check against the release channel's latest.json, and the hand-off to the verified release installer.
use serde_json::Value;

use crate::domain::config::{INSTALLER_ASSET, RELEASE_APP, RELEASE_DOWNLOAD_URL, RELEASE_TAG_PREFIX};
#[cfg(not(windows))]
use crate::domain::config::LATEST_URL;
use crate::domain::error::{FmError, R};
use crate::domain::version::compare_versions;
#[cfg(windows)]
pub use crate::platform::windows::updater::{latest_version, launch_updater};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub installer_url: String,
    pub sha256: String,
}

pub fn manifest_error(detail: impl std::fmt::Display) -> FmError {
    FmError::Internal { message: format!("The update channel's latest.json is not valid: {detail}") }
}

// Ok(None) is a schema this build does not know, which the release contract defines as "do not update".
pub fn parse_manifest(json: &str) -> R<Option<Release>> {
    let m: Value = serde_json::from_str(json).map_err(|e| manifest_error(format!("not JSON ({e})")))?;
    if m["schema"] != 1 {
        return if m["schema"].is_u64() { Ok(None) } else { Err(manifest_error("no schema")) };
    }
    if m["app"] != RELEASE_APP {
        return Err(manifest_error(format!("it is for {} instead of {RELEASE_APP}", m["app"])));
    }
    let version = m["version"].as_str().filter(|v| v.starts_with(|c: char| c.is_ascii_digit()) && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-'));
    let version = version.ok_or_else(|| manifest_error("no valid version"))?;
    let tag = format!("{RELEASE_TAG_PREFIX}v{version}");
    if m["tag"] != tag.as_str() {
        return Err(manifest_error(format!("the tag {} does not match {tag}", m["tag"])));
    }
    if m["prerelease"].as_bool() != Some(version.contains('-')) {
        return Err(manifest_error(format!("the prerelease flag does not match {version}")));
    }
    let asset = &m["assets"][INSTALLER_ASSET];
    let sha256 = asset["sha256"].as_str().filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())).ok_or_else(|| manifest_error(format!("no SHA-256 for {INSTALLER_ASSET}")))?;
    // Only the asset of this very release on github.com is accepted, whatever else the file names.
    let installer_url = format!("{RELEASE_DOWNLOAD_URL}{tag}/{INSTALLER_ASSET}");
    if asset["url"] != installer_url.as_str() {
        return Err(manifest_error(format!("the {INSTALLER_ASSET} URL is not {installer_url}")));
    }
    Ok(Some(Release { version: version.to_string(), installer_url, sha256: sha256.to_ascii_lowercase() }))
}

// Semver order, so a release beats its own -beta; the channel carries prereleases too.
pub fn is_newer(latest: &str, current: &str) -> bool {
    compare_versions(latest, current).is_gt()
}

// An unreachable channel means no update (it can 404 while CI replaces it); a broken one is an error.
pub fn newer_release(fetched: Option<String>) -> R<Option<Release>> {
    let Some(json) = fetched else { return Ok(None) };
    Ok(parse_manifest(&json)?.filter(|r| is_newer(&r.version, CURRENT_VERSION)))
}

#[cfg(not(windows))]
pub fn latest_version() -> R<Option<String>> {
    let out = std::process::Command::new("curl").args(["-fsSL", "--max-time", "10", "-A", "FolderManager-Updater", LATEST_URL]).output().ok();
    let fetched = out.filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
    Ok(newer_release(fetched)?.map(|r| r.version))
}

#[cfg(not(windows))]
pub fn launch_updater() -> R<()> {
    Err(FmError::Internal { message: "Automatic update is only available on Windows".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn manifest(version: &str, prerelease: bool, sha: &str) -> String {
        let tag = format!("folder-manager-v{version}");
        let url = format!("https://github.com/MikeWorldYt/ANT-system/releases/download/{tag}/FolderManager_installer.exe");
        format!(r#"{{"schema":1,"app":"FolderManager","version":"{version}","tag":"{tag}","prerelease":{prerelease},"commit":"abc","future":true,"assets":{{"FolderManager_portable.exe":{{"sha256":"{z}","url":"x"}},"FolderManager_installer.exe":{{"sha256":"{sha}","url":"{url}"}}}}}}"#, z = "0".repeat(64))
    }

    #[test]
    fn newer_compares_numbers_then_a_release_beats_its_prerelease() {
        assert!(is_newer("2.1.0", "2.0.1-beta"));
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(is_newer("2.0.1", "2.0.1-beta"));
        assert!(is_newer("2.4.6-beta", "2.4.5"));
        assert!(!is_newer("2.0.1-beta", "2.0.1"));
        assert!(!is_newer("2.4.5-beta", "2.4.5-beta"));
        assert!(!is_newer("1.3.0", "2.0.1-beta"));
        assert!(is_newer("2.4.5-beta.2", "2.4.5-beta.1"));
        assert!(is_newer("2.4.5-beta.1", "2.4.5-beta"));
        assert!(!is_newer("2.4.5-beta", "2.4.5-beta.1"));
        assert!(!is_newer("2.4.5-beta+b", "2.4.5-beta+a"));
    }

    #[test]
    fn the_installer_is_taken_from_a_valid_manifest() {
        let r = parse_manifest(&manifest("2.4.5-beta", true, &ABC.to_uppercase())).unwrap().unwrap();
        assert_eq!(r, Release { version: "2.4.5-beta".into(), installer_url: "https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.4.5-beta/FolderManager_installer.exe".into(), sha256: ABC.into() });
        assert_eq!(parse_manifest(&manifest("2.5.0", false, ABC)).unwrap().unwrap().version, "2.5.0");
    }

    #[test]
    fn an_unknown_schema_means_no_update() {
        assert_eq!(parse_manifest(&manifest("9.0.0", false, ABC).replace(r#""schema":1"#, r#""schema":2"#)).unwrap(), None);
    }

    #[test]
    fn a_broken_or_tampered_manifest_is_refused() {
        let good = manifest("2.4.5-beta", true, ABC);
        let bad = [
            "not json".to_string(),
            "{}".to_string(),
            good.replace(r#""schema":1"#, r#""schema":"1""#),
            good.replace(r#""app":"FolderManager""#, r#""app":"CreateNewStorage""#),
            good.replace(r#""prerelease":true"#, r#""prerelease":false"#),
            good.replace(r#""tag":"folder-manager-v2.4.5-beta""#, r#""tag":"folder-manager-v2.4.5""#),
            good.replace(r#""version":"2.4.5-beta""#, r#""version":"2.4.5-beta/../x""#),
            good.replace(ABC, &"0".repeat(63)),
            good.replace("https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.4.5-beta/FolderManager_installer.exe", "https://evil.example/FolderManager_installer.exe"),
            good.replace("FolderManager_installer.exe\":", "Other.exe\":"),
        ];
        for json in bad {
            assert!(parse_manifest(&json).is_err(), "{json}");
        }
    }

    #[test]
    fn only_a_newer_release_is_offered_and_an_unreachable_channel_is_no_update() {
        assert_eq!(newer_release(None).unwrap(), None);
        assert_eq!(newer_release(Some(manifest("0.1.0", false, ABC))).unwrap(), None);
        assert_eq!(newer_release(Some(manifest("99.0.0", false, ABC))).unwrap().unwrap().version, "99.0.0");
        assert!(newer_release(Some("<html>".into())).is_err());
    }

    #[test]
    #[ignore = "needs the network"]
    fn the_published_manifest_is_fetched_and_parsed() {
        let json = std::process::Command::new("curl").args(["-fsSL", crate::domain::config::LATEST_URL]).output().unwrap().stdout;
        assert!(parse_manifest(&String::from_utf8_lossy(&json)).unwrap().is_some());
    }
}
