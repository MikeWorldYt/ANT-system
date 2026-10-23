// Update check against the version published on main, and the hand-off to FM_Updater.bat.
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::ant_core::version::{parse_file_version, parse_version};
use crate::config::{CORE_FOLDER, EXE_DOWNLOAD_URL, SETTINGS_FOLDER, UPDATER_SCRIPT, VERSION_URL};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, run_powershell, CREATE_NO_WINDOW};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Same grace period the Python app used: an updater that dies sooner than this has failed.
const EARLY_EXIT: Duration = Duration::from_millis(1500);

// Offline or unparsable means no update, as in the Python app.
pub fn latest_version() -> Option<String> {
    let script = "(Invoke-WebRequest -UseBasicParsing -Uri $env:FM_URL -TimeoutSec 5 -Headers @{'User-Agent'='FolderManager-Updater'}).Content";
    let text = run_powershell(script, &[("FM_URL", VERSION_URL.as_ref())], None, Duration::from_secs(10)).ok()?;
    parse_file_version(&text)
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

// Starts FM_Updater.bat, which waits for this app to close before it swaps the executable.
pub fn launch_updater(root: &Path) -> R<()> {
    let core = root.join(SETTINGS_FOLDER).join(CORE_FOLDER);
    let bat = core.join(UPDATER_SCRIPT);
    if !bat.is_file() {
        return Err(FmError::Internal { message: format!("Could not find {UPDATER_SCRIPT} in {}", core.display()) });
    }
    let exe = std::env::current_exe().map_err(|e| io_err("find the app", &core, &e))?;
    // cmd /s strips only the outer quote pair, so every quoted path survives spaces.
    let mut child = Command::new("cmd")
        .raw_arg(format!(r#"/d /s /c ""{}" "{}" "{EXE_DOWNLOAD_URL}"""#, bat.display(), exe.display()))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| io_err("start the updater", &bat, &e))?;
    let started = Instant::now();
    while started.elapsed() < EARLY_EXIT {
        if let Some(status) = child.try_wait().map_err(|e| io_err("watch the updater", &bat, &e))? {
            return Err(FmError::Internal { message: format!("The updater stopped right away ({status}). FolderManager stays open") });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn newer_compares_numbers_and_ignores_the_prefix_and_beta_tag() {
        assert!(is_newer("v2.1.0", "2.0.1-beta"));
        assert!(is_newer("v1.10.0", "1.9.9"));
        assert!(!is_newer("v2.0.1", "2.0.1-beta"));
        assert!(!is_newer("v1.3.0", "2.0.1-beta"));
    }

    #[test]
    fn a_storage_without_the_updater_script_is_reported() {
        let dir = TempDir::new("updater-missing");
        let err = launch_updater(dir.path()).unwrap_err();
        assert!(err.to_string().contains(UPDATER_SCRIPT), "{err}");
    }

    #[test]
    fn an_updater_that_exits_at_once_is_reported_as_failed() {
        let dir = TempDir::new("updater-fails");
        dir.mkdir("999.Settings/00.Core");
        std::fs::write(dir.path().join("999.Settings/00.Core/FM_Updater.bat"), "@exit /b 7\r\n").unwrap();
        let err = launch_updater(dir.path()).unwrap_err();
        assert!(err.to_string().contains("stopped right away"), "{err}");
    }

    #[test]
    #[ignore = "needs the network"]
    fn the_published_version_is_fetched_and_parsed() {
        let latest = latest_version().expect("version.txt on main could not be fetched");
        assert!(latest.starts_with('v') && parse_version(&latest) > [0, 0, 0], "{latest}");
    }
}
