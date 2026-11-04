// SHA-256 and unzip through the tools each system already ships, so no crate is needed.
use std::path::Path;

use super::fail;
use super::fetch::valid_sha256;
use crate::error::R;

pub fn hash_from(output: &str) -> R<String> {
    let hash = output.split_whitespace().next().unwrap_or_default().to_ascii_lowercase();
    if !valid_sha256(&hash) {
        return Err(fail("Could not compute the sha256 of the download".to_string()));
    }
    Ok(hash)
}

#[cfg(windows)]
pub fn sha256_file(file: &Path) -> R<String> {
    let script = "$ErrorActionPreference = 'Stop'; (Get-FileHash -Algorithm SHA256 -LiteralPath $env:FM_FILE).Hash";
    let out = crate::fs_safe::run_powershell(script, &[("FM_FILE", file.as_os_str())], None, std::time::Duration::from_secs(60))
        .map_err(|e| fail(format!("Could not compute the sha256: {e}")))?;
    hash_from(&out)
}

#[cfg(not(windows))]
pub fn sha256_file(file: &Path) -> R<String> {
    let tools: [(&str, &[&str]); 2] = [("sha256sum", &[]), ("shasum", &["-a", "256"])];
    let mut last = String::new();
    for (exe, args) in tools {
        match std::process::Command::new(exe).args(args).arg(file).output() {
            Ok(out) if out.status.success() => return hash_from(&String::from_utf8_lossy(&out.stdout)),
            Ok(out) => last = String::from_utf8_lossy(&out.stderr).trim().to_string(),
            Err(e) => last = format!("{exe}: {e}"),
        }
    }
    Err(fail(format!("Could not compute the sha256: {last}")))
}

#[cfg(windows)]
pub fn expand(zip: &Path, dest: &Path) -> R<()> {
    let script = "$ErrorActionPreference = 'Stop'; Add-Type -AssemblyName System.IO.Compression.FileSystem; \
        [IO.Compression.ZipFile]::ExtractToDirectory($env:FM_ZIP, $env:FM_DEST)";
    let env = [("FM_ZIP", zip.as_os_str()), ("FM_DEST", dest.as_os_str())];
    crate::fs_safe::run_powershell(script, &env, None, std::time::Duration::from_secs(120))
        .map(|_| ())
        .map_err(|e| fail(format!("Could not expand the extension: {e}")))
}

#[cfg(not(windows))]
pub fn expand(zip: &Path, dest: &Path) -> R<()> {
    match std::process::Command::new("unzip").arg("-q").arg(zip).arg("-d").arg(dest).output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(fail(format!("Could not expand the extension: {}", String::from_utf8_lossy(&out.stderr).trim()))),
        Err(e) => Err(fail(format!("Could not expand the extension: unzip is not available ({e})"))),
    }
}
