// Creates .lnk shortcuts in one PowerShell run through the WScript.Shell COM object.
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
