// Update check against the version published on main, and the hand-off to FM_Updater.bat.
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::time::{Duration, Instant};

use crate::ant_core::version::{parse_file_version, parse_version};
use crate::config::VERSION_URL;
#[cfg(windows)]
use crate::config::{CORE_FOLDER, EXE_DOWNLOAD_URL, SETTINGS_FOLDER, UPDATER_SCRIPT, UPDATER_SCRIPT_URL};
use crate::error::{FmError, R};
#[cfg(windows)]
use crate::fs_safe::{io_err, run_powershell, write_atomic, CREATE_NO_WINDOW};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Same grace period the Python app used: an updater that dies sooner than this has failed.
#[cfg(windows)]
const EARLY_EXIT: Duration = Duration::from_millis(1500);

// Offline or unparsable means no update, as in the Python app.
#[cfg(windows)]
pub fn latest_version() -> Option<String> {
    let script = "(Invoke-WebRequest -UseBasicParsing -Uri $env:FM_URL -TimeoutSec 5 -Headers @{'User-Agent'='FolderManager-Updater'}).Content";
    let text = run_powershell(script, &[("FM_URL", VERSION_URL.as_ref())], None, Duration::from_secs(10)).ok()?;
    parse_file_version(&text)
}

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

// Starts FM_Updater.bat, which waits for this app to close before it swaps the executable.
#[cfg(windows)]
pub fn launch_updater(root: &Path) -> R<()> {
    let core = root.join(SETTINGS_FOLDER).join(CORE_FOLDER);
    let bat = core.join(UPDATER_SCRIPT);
    if !bat.is_file() {
        fetch_updater(&core, UPDATER_SCRIPT_URL)?;
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

// The published script is saved with CRLF endings, which cmd needs for its labels and jumps.
#[cfg(windows)]
fn fetch_updater(core: &Path, url: &str) -> R<()> {
    std::fs::create_dir_all(core).map_err(|e| io_err("create the core folder", core, &e))?;
    let bat = core.join(UPDATER_SCRIPT);
    let part = core.join(format!("{UPDATER_SCRIPT}.part"));
    let out = part.display().to_string();
    let script = "Invoke-WebRequest -UseBasicParsing -Uri $env:FM_URL -OutFile $env:FM_OUT -TimeoutSec 30 -Headers @{'User-Agent'='FolderManager-Updater'}";
    run_powershell(script, &[("FM_URL", url.as_ref()), ("FM_OUT", out.as_ref())], None, Duration::from_secs(45))
        .map_err(|e| FmError::Internal { message: format!("Could not download {UPDATER_SCRIPT}: {e}") })?;
    let text = std::fs::read_to_string(&part).map_err(|e| io_err("read the download", &part, &e))?;
    if text.trim().is_empty() {
        return Err(FmError::Internal { message: format!("Could not download {UPDATER_SCRIPT}: the file is empty") });
    }
    write_atomic(&bat, to_crlf(&text).as_bytes())?;
    std::fs::remove_file(&part).map_err(|e| io_err("remove the download", &part, &e))
}

#[cfg(windows)]
fn to_crlf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

#[cfg(not(windows))]
pub fn launch_updater(_root: &Path) -> R<()> {
    Err(FmError::Internal { message: "Automatic update is only available on Windows".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use crate::test_support::TempDir;

    #[test]
    fn newer_compares_numbers_and_ignores_the_prefix_and_beta_tag() {
        assert!(is_newer("v2.1.0", "2.0.1-beta"));
        assert!(is_newer("v1.10.0", "1.9.9"));
        assert!(!is_newer("v2.0.1", "2.0.1-beta"));
        assert!(!is_newer("v1.3.0", "2.0.1-beta"));
    }

    #[cfg(windows)]
    #[test]
    fn a_missing_updater_script_that_cannot_be_downloaded_is_reported() {
        let dir = TempDir::new("updater-missing");
        let core = dir.path().join("999.Settings/00.Core");
        let err = fetch_updater(&core, "http://127.0.0.1:9/FM_Updater.bat").unwrap_err();
        assert!(err.to_string().contains("Could not download"), "{err}");
        assert!(!core.join(UPDATER_SCRIPT).exists());
    }

    #[cfg(windows)]
    #[test]
    fn the_downloaded_script_is_saved_with_crlf_endings() {
        assert_eq!(to_crlf("@echo off\n:: x\r\n"), "@echo off\r\n:: x\r\n");
    }

    #[cfg(windows)]
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
