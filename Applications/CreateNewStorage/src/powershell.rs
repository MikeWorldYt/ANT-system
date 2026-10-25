// Hidden, time-limited PowerShell for the shell tasks std has no API for.
use std::path::PathBuf;
use std::time::Duration;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// Paths travel as environment variables and stdin, never inside the script text.
pub fn run_powershell(
    script: &str,
    env: &[(&str, &std::ffi::OsStr)],
    stdin: Option<String>,
    timeout: Duration,
) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    // An absolute path keeps a powershell.exe planted next to the app or on PATH from running.
    let exe = std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join(r"System32\WindowsPowerShell\v1.0\powershell.exe"))
        .unwrap_or_else(|| PathBuf::from("powershell.exe"));
    let mut child = Command::new(exe)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .envs(env.iter().copied())
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Could not start PowerShell: {e}"))?;
    // Writing on another thread avoids a deadlock when PowerShell fills stdout before reading all input.
    let writer = match (stdin, child.stdin.take()) {
        (Some(text), Some(mut pipe)) => Some(std::thread::spawn(move || pipe.write_all(text.as_bytes()))),
        _ => None,
    };
    type Reader = std::thread::JoinHandle<std::io::Result<Vec<u8>>>;
    fn drain<P: Read + Send + 'static>(pipe: Option<P>) -> Option<Reader> {
        pipe.map(|mut p| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                p.read_to_end(&mut bytes).map(|_| bytes)
            })
        })
    }
    let out_reader = drain(child.stdout.take());
    let err_reader = drain(child.stderr.take());
    // A shell dialog nobody answers would otherwise block this thread forever.
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let stopped = child.kill().and_then(|_| child.wait());
                return Err(match stopped {
                    Ok(_) => format!("PowerShell did not finish within {} s and was stopped", timeout.as_secs()),
                    Err(e) => format!("PowerShell did not finish within {} s and could not be stopped: {e}", timeout.as_secs()),
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(format!("PowerShell did not finish: {e}")),
        }
    };
    if let Some(handle) = writer {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(format!("Could not send input to PowerShell: {e}")),
            Err(_) => return Err("PowerShell input writer panicked".to_string()),
        }
    }
    let collect = |reader: Option<Reader>| match reader {
        Some(handle) => match handle.join() {
            Ok(Ok(bytes)) => Ok(String::from_utf8_lossy(&bytes).trim().to_string()),
            Ok(Err(e)) => Err(format!("Could not read PowerShell output: {e}")),
            Err(_) => Err("PowerShell output reader panicked".to_string()),
        },
        None => Ok(String::new()),
    };
    let stdout = collect(out_reader)?;
    let stderr = collect(err_reader)?;
    if status.success() {
        return Ok(stdout);
    }
    let detail = if stdout.is_empty() { stderr } else { stdout };
    Err(format!("PowerShell failed ({status}): {detail}"))
}
