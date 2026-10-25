// Zips a staged backup and expands one back, with the tools each system already ships.
use std::path::Path;

use crate::error::{FmError, R};

fn fail(op: &str, detail: String) -> FmError {
    FmError::Internal { message: format!("Could not {op} the backup: {detail}") }
}

// ZipFile keeps dot-folders and hidden files that Compress-Archive's wildcard would drop.
#[cfg(windows)]
const COMPRESS: &str = "$ErrorActionPreference = 'Stop'; Add-Type -AssemblyName System.IO.Compression.FileSystem; \
    [IO.Compression.ZipFile]::CreateFromDirectory($env:FM_STAGE, $env:FM_ZIP)";
#[cfg(windows)]
const EXPAND: &str = "$ErrorActionPreference = 'Stop'; Add-Type -AssemblyName System.IO.Compression.FileSystem; \
    [IO.Compression.ZipFile]::ExtractToDirectory($env:FM_ZIP, $env:FM_STAGE)";

#[cfg(windows)]
fn run(op: &str, script: &str, stage: &Path, zip: &Path) -> R<()> {
    let env = [("FM_STAGE", stage.as_os_str()), ("FM_ZIP", zip.as_os_str())];
    crate::fs_safe::run_powershell(script, &env, None, std::time::Duration::from_secs(120))
        .map(|_| ())
        .map_err(|e| fail(op, e))
}

#[cfg(windows)]
pub fn compress(stage: &Path, zip: &Path) -> R<()> {
    run("compress", COMPRESS, stage, zip)
}

#[cfg(windows)]
pub fn expand(zip: &Path, stage: &Path) -> R<()> {
    run("expand", EXPAND, stage, zip)
}

#[cfg(not(windows))]
fn run(op: &str, cmd: &mut std::process::Command) -> R<()> {
    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(fail(op, String::from_utf8_lossy(&out.stderr).trim().to_string())),
        Err(e) => Err(fail(op, format!("zip/unzip is not available ({e})"))),
    }
}

#[cfg(not(windows))]
pub fn compress(stage: &Path, zip: &Path) -> R<()> {
    run("compress", std::process::Command::new("zip").arg("-qr").arg(zip).arg(".").current_dir(stage))
}

#[cfg(not(windows))]
pub fn expand(zip: &Path, stage: &Path) -> R<()> {
    run("expand", std::process::Command::new("unzip").arg("-q").arg(zip).arg("-d").arg(stage))
}
