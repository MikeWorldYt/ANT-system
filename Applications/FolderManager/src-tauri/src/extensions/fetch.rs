// Downloads through curl, kept to https and a short list of GitHub hosts.
use std::process::Command;

use super::fail;
use crate::error::R;

pub const REGISTRY_HOSTS: [&str; 3] = ["github.com", "raw.githubusercontent.com", "objects.githubusercontent.com"];
// The zipball API answers on api.github.com and redirects to codeload.github.com.
pub const REPO_HOSTS: [&str; 2] = ["api.github.com", "codeload.github.com"];
pub const MAX_ZIP_BYTES: u64 = 5 * 1024 * 1024;
const MAX_URL_CHARS: usize = 2048;
const REPO_PREFIX: &str = "https://github.com/";

pub fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains(['@', ':'])).then_some(host)
}

pub fn allowed_url(url: &str, hosts: &[&str]) -> bool {
    url.len() <= MAX_URL_CHARS
        && url.chars().all(|c| c.is_ascii_graphic() && !"\"\\<>`{}|^[]".contains(c))
        && host_of(url).is_some_and(|h| hosts.iter().any(|a| a.eq_ignore_ascii_case(h)))
}

pub fn valid_sha256(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn same_hash(a: &str, b: &str) -> bool {
    valid_sha256(a) && valid_sha256(b) && a.eq_ignore_ascii_case(b)
}

fn valid_repo_part(part: &str) -> bool {
    !part.is_empty() && part != "." && part != ".." && part.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

pub fn parse_repo_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix(REPO_PREFIX)?;
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let (owner, repo) = rest.split_once('/')?;
    (valid_repo_part(owner) && valid_repo_part(repo)).then(|| (owner.to_string(), repo.to_string()))
}

pub fn repo_url(owner: &str, repo: &str) -> String {
    format!("{REPO_PREFIX}{owner}/{repo}")
}

pub fn curl_message(code: Option<i32>) -> String {
    match code {
        Some(6 | 7 | 28 | 35) => "GitHub could not be reached; check the connection".to_string(),
        Some(22) => "GitHub answered with an error (missing or private?)".to_string(),
        Some(63) => "The download is larger than allowed".to_string(),
        Some(47) => "The download was redirected too many times".to_string(),
        Some(1) => "The download was redirected to a non-https address".to_string(),
        Some(n) => format!("The download failed (curl exit code {n})"),
        None => "The download was interrupted".to_string(),
    }
}

// The effective address is written after the body, behind the last newline.
pub fn split_effective(out: &[u8]) -> Option<(&[u8], &str)> {
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
    cmd.creation_flags(crate::fs_safe::CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn curl() -> Command {
    Command::new("curl")
}

pub fn download(url: &str, max: u64, seconds: u32, hosts: &[&str], headers: &[&str]) -> R<Vec<u8>> {
    if !allowed_url(url, hosts) {
        return Err(fail(format!("{url} is not an allowed address")));
    }
    let mut cmd = curl();
    cmd.args(["-fsSL", "-g", "--proto", "=https", "--proto-redir", "=https", "--max-redirs", "5"])
        .args(["--max-time", &seconds.to_string(), "--max-filesize", &max.to_string(), "-w", "\n%{url_effective}"]);
    for header in headers {
        cmd.args(["-H", header]);
    }
    let out = cmd.arg(url).output().map_err(|e| fail(format!("curl is not available ({e})")))?;
    if !out.status.success() {
        return Err(fail(curl_message(out.status.code())));
    }
    let (body, effective) = split_effective(&out.stdout).ok_or_else(|| fail("The download returned nothing".to_string()))?;
    if !allowed_url(effective, hosts) {
        return Err(fail("The download was redirected to a host that is not allowed".to_string()));
    }
    if body.len() as u64 > max {
        return Err(fail(curl_message(Some(63))));
    }
    Ok(body.to_vec())
}

pub fn repo_zip(owner: &str, repo: &str) -> R<Vec<u8>> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/zipball");
    let headers = ["User-Agent: ANT-System-Manager", "Accept: application/vnd.github+json"];
    download(&url, MAX_ZIP_BYTES, 60, &REPO_HOSTS, &headers)
}
