// Windows update path: the version check and FM_Updater.bat run through PowerShell and cmd.
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::domain::config::{EXE_DOWNLOAD_URL, UPDATER_SCRIPT, UPDATER_SCRIPT_URL, VERSION_URL};
use crate::infra::storage::core_dir::{data_dir, root_file};
use crate::domain::error::{FmError, R};
use crate::domain::version::parse_file_version;
use crate::infra::fs_safe::{io_err, write_atomic};
use crate::platform::windows::powershell::{run_powershell, CREATE_NO_WINDOW};

// Same grace period the Python app used: an updater that dies sooner than this has failed.
const EARLY_EXIT: Duration = Duration::from_millis(1500);

// Offline or unparsable means no update, as in the Python app.
pub fn latest_version() -> Option<String> {
    let script = "(Invoke-WebRequest -UseBasicParsing -Uri $env:FM_URL -TimeoutSec 5 -Headers @{'User-Agent'='FolderManager-Updater'}).Content";
    let text = run_powershell(script, &[("FM_URL", VERSION_URL.as_ref())], None, Duration::from_secs(10)).ok()?;
    parse_file_version(&text)
}

// Starts FM_Updater.bat, which waits for this app to close before it swaps the executable.
pub fn launch_updater(root: &Path) -> R<()> {
    let bat = root_file(root, UPDATER_SCRIPT);
    if !bat.is_file() {
        fetch_updater(&data_dir(root), UPDATER_SCRIPT_URL)?;
    }
    let exe = std::env::current_exe().map_err(|e| io_err("find the app", &bat, &e))?;
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

fn to_crlf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_missing_updater_script_that_cannot_be_downloaded_is_reported() {
        let dir = TempDir::new("updater-missing");
        let core = dir.path().join(".ant");
        let err = fetch_updater(&core, "http://127.0.0.1:9/FM_Updater.bat").unwrap_err();
        assert!(err.to_string().contains("Could not download"), "{err}");
        assert!(!core.join(UPDATER_SCRIPT).exists());
    }

    #[test]
    fn the_downloaded_script_is_saved_with_crlf_endings() {
        assert_eq!(to_crlf("@echo off\n:: x\r\n"), "@echo off\r\n:: x\r\n");
    }

    #[test]
    fn an_updater_that_exits_at_once_is_reported_as_failed() {
        let dir = TempDir::new("updater-fails");
        dir.mkdir(".ant");
        std::fs::write(dir.path().join(".ant/FM_Updater.bat"), "@exit /b 7\r\n").unwrap();
        let err = launch_updater(dir.path()).unwrap_err();
        assert!(err.to_string().contains("stopped right away"), "{err}");
    }
}
