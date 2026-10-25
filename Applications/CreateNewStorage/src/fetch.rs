// Downloads through the system curl, kept to https and a short list of GitHub hosts.
use std::process::Command;

pub const GITHUB_HOSTS: [&str; 4] = ["github.com", "raw.githubusercontent.com", "objects.githubusercontent.com", "api.github.com"];
pub const MAX_TEXT_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_EXE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_URL_CHARS: usize = 2048;

fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains(['@', ':'])).then_some(host)
}

pub fn allowed_url(url: &str) -> bool {
    url.len() <= MAX_URL_CHARS
        && url.chars().all(|c| c.is_ascii_graphic() && !"\"\\<>`{}|^[]".contains(c))
        && host_of(url).is_some_and(|h| GITHUB_HOSTS.iter().any(|a| a.eq_ignore_ascii_case(h)))
}

fn curl_message(code: Option<i32>) -> String {
    match code {
        Some(6 | 7 | 28 | 35) => "GitHub could not be reached; check the connection".to_string(),
        Some(22) => "GitHub answered with an error".to_string(),
        Some(63) => "The download is larger than allowed".to_string(),
        Some(47) => "The download was redirected too many times".to_string(),
        Some(1) => "The download was redirected to a non-https address".to_string(),
        Some(n) => format!("The download failed (curl exit code {n})"),
        None => "The download was interrupted".to_string(),
    }
}

// The effective address is written after the body, behind the last newline.
fn split_effective(out: &[u8]) -> Option<(&[u8], &str)> {
    let at = out.iter().rposition(|&b| b == b'\n')?;
    Some((&out[..at], std::str::from_utf8(&out[at + 1..]).ok()?))
}

// An absolute path keeps a curl.exe planted next to the app or on PATH from running.
#[cfg(windows)]
fn curl() -> Command {
    use std::os::windows::process::CommandExt;
    let exe = std::env::var_os("SystemRoot")
        .map(|r| std::path::PathBuf::from(r).join(r"System32\curl.exe"))
        .unwrap_or_else(|| std::path::PathBuf::from("curl.exe"));
    let mut cmd = Command::new(exe);
    cmd.creation_flags(crate::powershell::CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn curl() -> Command {
    Command::new("curl")
}

pub fn download(url: &str, max: u64, seconds: u32) -> Result<Vec<u8>, String> {
    if !allowed_url(url) {
        return Err(format!("{url} is not an allowed address"));
    }
    let out = curl()
        .args(["-fsSL", "-g", "--proto", "=https", "--proto-redir", "=https", "--max-redirs", "5"])
        .args(["--max-time", &seconds.to_string(), "--max-filesize", &max.to_string(), "-w", "\n%{url_effective}"])
        .args(["-H", crate::config::USER_AGENT])
        .arg(url)
        .output()
        .map_err(|e| format!("curl is not available ({e})"))?;
    if !out.status.success() {
        return Err(curl_message(out.status.code()));
    }
    let (body, effective) = split_effective(&out.stdout).ok_or("The download returned nothing")?;
    if !allowed_url(effective) {
        return Err("The download was redirected to a host that is not allowed".to_string());
    }
    if body.len() as u64 > max {
        return Err(curl_message(Some(63)));
    }
    Ok(body.to_vec())
}

pub fn download_text(url: &str) -> Result<String, String> {
    String::from_utf8(download(url, MAX_TEXT_BYTES, 30)?).map_err(|_| format!("{url} is not UTF-8 text"))
}

// GitHub's contents API answers JSON, which the YAML parser reads as flow style.
pub fn core_file_list(listing: &str) -> Result<Vec<(String, String)>, String> {
    let items: Vec<serde_yaml::Value> = serde_yaml::from_str(listing).map_err(|e| format!("Unexpected core file listing: {e}"))?;
    let mut files = Vec::new();
    for item in items.iter().filter(|i| i.get("type").and_then(|t| t.as_str()) == Some("file")) {
        let field = |k: &str| item.get(k).and_then(|v| v.as_str()).ok_or(format!("Core file entry without {k}"));
        let name = field("name")?;
        if name.contains(['/', '\\', '\0']) || name == ".." || name == "." {
            return Err(format!("Core file has an invalid name: {name:?}"));
        }
        files.push((name.to_string(), field("download_url")?.to_string()));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_github_hosts_are_allowed() {
        assert!(allowed_url("https://raw.githubusercontent.com/a/b"));
        assert!(!allowed_url("http://github.com/a"));
        assert!(!allowed_url("https://evil.com/github.com"));
        assert!(!allowed_url("https://user@github.com/a"));
    }

    #[test]
    fn core_list_keeps_files_and_rejects_bad_names() {
        let json = r#"[{"name":"Tree.yaml","type":"file","download_url":"https://x/1"},{"name":"d","type":"dir","download_url":null}]"#;
        assert_eq!(core_file_list(json).unwrap(), vec![("Tree.yaml".to_string(), "https://x/1".to_string())]);
        assert!(core_file_list(r#"[{"name":"../a","type":"file","download_url":"u"}]"#).is_err());
    }
}
