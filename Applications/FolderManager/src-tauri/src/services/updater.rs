// Update check against the version published on main, and the hand-off to FM_Updater.bat.
#[cfg(not(windows))]
use std::path::Path;

#[cfg(not(windows))]
use crate::domain::version::parse_file_version;
use crate::domain::version::parse_version;
#[cfg(not(windows))]
use crate::domain::config::VERSION_URL;
#[cfg(not(windows))]
use crate::domain::error::{FmError, R};
#[cfg(windows)]
pub use crate::platform::windows::updater::{latest_version, launch_updater};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(windows))]
pub fn latest_version() -> Option<String> {
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "10", "-A", "FolderManager-Updater", VERSION_URL])
        .output()
        .ok()?;
    out.status.success().then(|| parse_file_version(&String::from_utf8_lossy(&out.stdout)))?
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

#[cfg(not(windows))]
pub fn launch_updater(_root: &Path) -> R<()> {
    Err(FmError::Internal { message: "Automatic update is only available on Windows".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_compares_numbers_and_ignores_the_prefix_and_beta_tag() {
        assert!(is_newer("v2.1.0", "2.0.1-beta"));
        assert!(is_newer("v1.10.0", "1.9.9"));
        assert!(!is_newer("v2.0.1", "2.0.1-beta"));
        assert!(!is_newer("v1.3.0", "2.0.1-beta"));
    }

    #[test]
    #[ignore = "needs the network"]
    fn the_published_version_is_fetched_and_parsed() {
        let latest = latest_version().expect("version.txt on main could not be fetched");
        assert!(latest.starts_with('v') && parse_version(&latest) > [0, 0, 0], "{latest}");
    }
}
