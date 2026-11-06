// Downloads through the system curl, kept to https and a short list of hosts.
use std::process::Command;

pub const ALLOWED_HOSTS: [&str; 6] = [
    "github.com",
    "raw.githubusercontent.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
    "api.github.com",
    "r2.mikeworld.dev",
];
pub const MAX_TEXT_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_EXE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_URL_CHARS: usize = 2048;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains(['@', ':'])).then_some(host)
}

pub fn allowed_url(url: &str) -> bool {
    url.len() <= MAX_URL_CHARS
        && url.chars().all(|c| c.is_ascii_graphic() && !"\"\\<>`{}|^[]".contains(c))
        && host_of(url).is_some_and(|h| ALLOWED_HOSTS.iter().any(|a| a.eq_ignore_ascii_case(h)))
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
    cmd.creation_flags(CREATE_NO_WINDOW);
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

// Highest stable `<prefix>X.Y.Z` tag in a releases listing (pre-releases like `-rc1` are skipped). The text is scanned
// for "tag_name" instead of parsed, because only this one field is needed.
pub fn latest_release_tag(listing: &str, prefix: &str) -> Option<String> {
    let mut best: Option<((u64, u64, u64), String)> = None;
    for part in listing.split("\"tag_name\"").skip(1) {
        let Some(value) = part.trim_start().strip_prefix(':').map(str::trim_start).and_then(|v| v.strip_prefix('"')) else { continue };
        let Some(tag) = value.split('"').next() else { continue };
        let Some(version) = tag.strip_prefix(prefix).and_then(parse_version) else { continue };
        if best.as_ref().map_or(true, |(top, _)| version > *top) {
            best = Some((version, tag.to_string()));
        }
    }
    best.map(|(_, tag)| tag)
}

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
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
    fn release_assets_hosts_are_allowed() {
        assert!(allowed_url("https://release-assets.githubusercontent.com/github-production-release-asset/1"));
    }

    #[test]
    fn latest_tag_picks_the_highest_stable_version_of_the_prefix() {
        let json = r#"[{"tag_name":"core-v9.0.0"},{"tag_name":"folder-manager-v2.4.0","body":"say \"tag_name\" \ud83d\ude00"},{"tag_name":"folder-manager-v2.10.1"},{"tag_name":"folder-manager-v3.0.0-rc1"},{"tag_name":"folder-manager-v2.9.9"}]"#;
        assert_eq!(latest_release_tag(json, "folder-manager-v").as_deref(), Some("folder-manager-v2.10.1"));
        assert_eq!(latest_release_tag("[]", "folder-manager-v"), None);
        assert_eq!(latest_release_tag(r#"[{"tag_name": "folder-manager-v1.2.3"}]"#, "folder-manager-v").as_deref(), Some("folder-manager-v1.2.3"));
    }
}
