// Creates and reads .lnk shortcuts in one PowerShell run through the WScript.Shell COM object.
use std::path::PathBuf;

const SHORTCUT_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $in = [IO.StreamReader]::new([Console]::OpenStandardInput(), [Text.UTF8Encoding]::new($false)); \
    $out = [IO.StreamWriter]::new([Console]::OpenStandardOutput(), [Text.UTF8Encoding]::new($false)); \
    $shell = New-Object -ComObject WScript.Shell; $i = 0; \
    while ($null -ne ($line = $in.ReadLine())) { \
        try { $p = $line.Split([char]9); $s = $shell.CreateShortcut($p[1]); $s.TargetPath = $p[0]; $s.Save() } \
        catch { $out.WriteLine([string]$i + [char]9 + $_.Exception.Message) }; \
        $i++ }; \
    $out.Flush()";

const TARGET_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $in = [IO.StreamReader]::new([Console]::OpenStandardInput(), [Text.UTF8Encoding]::new($false)); \
    $out = [IO.StreamWriter]::new([Console]::OpenStandardOutput(), [Text.UTF8Encoding]::new($false)); \
    $shell = New-Object -ComObject WScript.Shell; $i = 0; \
    while ($null -ne ($line = $in.ReadLine())) { \
        try { $target = $shell.CreateShortcut($line).TargetPath } catch { $target = '' }; \
        $out.WriteLine([string]$i + ':' + $target); \
        $i++ }; \
    $out.Flush()";

pub(crate) fn platform_targets(links: &[PathBuf]) -> Result<Vec<String>, String> {
    if links.is_empty() {
        return Ok(Vec::new());
    }
    let input: String = links.iter().map(|link| format!("{}\n", link.display())).collect();
    let stdout = crate::platform::windows::powershell::run_powershell(TARGET_SCRIPT, &[], Some(input), std::time::Duration::from_secs(120))?;
    paired_targets(&stdout, links.len())
}

// An empty target would be a blank line that run_powershell trims away, and links are deleted by this pairing, so each line names its link and any mismatch fails.
pub(crate) fn paired_targets(stdout: &str, links: usize) -> Result<Vec<String>, String> {
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.trim().is_empty()).collect();
    if lines.len() != links {
        return Err(format!("expected {links} shortcut targets, got {}", lines.len()));
    }
    let paired = |(i, line): (usize, &&str)| match line.split_once(':') {
        Some((index, target)) if index.parse() == Ok(i) => Ok(target.to_string()),
        _ => Err(format!("shortcut target {i} does not match its link: {line}")),
    };
    lines.iter().enumerate().map(paired).collect()
}

pub(crate) fn platform_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    let mut failures = Vec::new();
    let mut sent: Vec<PathBuf> = Vec::new();
    let mut input = String::new();
    for (target, link) in pairs {
        let mut link = link.clone().into_os_string();
        link.push(".lnk");
        let link = PathBuf::from(link);
        let (t, l) = (target.to_string_lossy(), link.to_string_lossy());
        // Windows names cannot hold tabs or newlines, so these only appear in mangled input.
        if [&t, &l].iter().any(|s| s.contains(['\t', '\n', '\r', '\u{FFFD}'])) {
            failures.push(format!("Shortcut not created: {}: unsupported characters", link.display()));
            continue;
        }
        input.push_str(&format!("{t}\t{l}\n"));
        sent.push(link);
    }
    if sent.is_empty() {
        return failures;
    }
    let timeout = std::time::Duration::from_secs(120);
    match crate::platform::windows::powershell::run_powershell(SHORTCUT_SCRIPT, &[], Some(input), timeout) {
        Ok(stdout) => {
            for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
                let (index, message) = line.split_once('\t').unwrap_or(("", line));
                let link = index.trim().parse::<usize>().ok().and_then(|i| sent.get(i));
                match link {
                    Some(link) => failures.push(format!("Shortcut not created: {}: {message}", link.display())),
                    None => failures.push(format!("Shortcut not created: {line}")),
                }
            }
        }
        Err(e) => failures.extend(sent.iter().map(|l| format!("Shortcut not created: {}: {e}", l.display()))),
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    #[ignore = "starts PowerShell and creates real .lnk files"]
    fn a_corrupt_link_listed_first_keeps_its_own_empty_target() {
        let dir = TempDir::new("targets-corrupt");
        let corrupt = dir.path().join("a.lnk");
        std::fs::write(&corrupt, "not a shortcut").unwrap();
        let folder = dir.path().join("folder");
        std::fs::create_dir(&folder).unwrap();
        assert_eq!(platform_create(&[(folder.clone(), dir.path().join("b"))]), Vec::<String>::new());
        let targets = platform_targets(&[corrupt, dir.path().join("b.lnk")]).unwrap();
        assert_eq!(targets, [String::new(), folder.display().to_string()]);
    }
}
