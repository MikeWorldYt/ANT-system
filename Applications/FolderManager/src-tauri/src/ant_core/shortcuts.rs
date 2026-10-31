// Names of the 998.Review shortcuts, derived from the owner path so review links keep the owner's numbering.
use std::path::PathBuf;

use crate::ant_core::names::editable_part;
use crate::error::{FmError, R};

#[cfg(windows)]
const SHORTCUT_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $in = [IO.StreamReader]::new([Console]::OpenStandardInput(), [Text.UTF8Encoding]::new($false)); \
    $out = [IO.StreamWriter]::new([Console]::OpenStandardOutput(), [Text.UTF8Encoding]::new($false)); \
    $shell = New-Object -ComObject WScript.Shell; $i = 0; \
    while ($null -ne ($line = $in.ReadLine())) { \
        try { $p = $line.Split([char]9); $s = $shell.CreateShortcut($p[1]); $s.TargetPath = $p[0]; $s.Save() } \
        catch { $out.WriteLine([string]$i + [char]9 + $_.Exception.Message) }; \
        $i++ }; \
    $out.Flush()";

// Pairs are (target folder, link path without extension); Windows gets `.lnk` files, other systems symlinks.
pub fn create_shortcuts(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    if pairs.is_empty() {
        return Vec::new();
    }
    platform_create(pairs)
}

#[cfg(windows)]
fn platform_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
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
    match crate::fs_safe::run_powershell(SHORTCUT_SCRIPT, &[], Some(input), timeout) {
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

#[cfg(not(windows))]
fn platform_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    let mut failures = Vec::new();
    for (target, link) in pairs {
        if std::fs::symlink_metadata(link).is_ok() {
            continue;
        }
        if let Err(e) = std::os::unix::fs::symlink(target, link) {
            failures.push(format!("Shortcut not created: {}: {e}", link.display()));
        }
    }
    failures
}

// Names sort like the tree: the category, then one two-digit slot per level below the second, "00" where the owner is shallower.
pub fn review_shortcut_name(owner: &[String], levels: usize) -> R<String> {
    if owner.len() > levels {
        return Err(FmError::InvalidPath {
            rel_parts: owner.to_vec(),
            reason: "No review shortcut below the storage's levels".to_string(),
        });
    }
    match owner {
        [] => Err(FmError::Internal {
            message: "A review shortcut needs an owner folder".to_string(),
        }),
        [only] => Ok(only.clone()),
        [first, second, rest @ ..] => {
            let mut name = format!("{}{}", first_char(first, owner)?, slot(second));
            for i in 0..levels.saturating_sub(2) {
                name.push('.');
                name.push_str(rest.get(i).map_or("00", |part| slot(part)));
            }
            name.push('.');
            name.push_str(editable_part(&owner[owner.len() - 1]));
            Ok(name)
        }
    }
}

fn slot(folder: &str) -> &str {
    folder.split('.').next().unwrap_or("")
}

fn first_char(folder: &str, owner: &[String]) -> R<char> {
    folder.chars().next().ok_or_else(|| FmError::InvalidPath {
        rel_parts: owner.to_vec(),
        reason: "Empty folder name in review owner path".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn review_shortcut_name_follows_owner_depth() {
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC"]), 3).unwrap(), "400.ACADEMIC");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math"]), 3).unwrap(), "401.00.Math");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "01.Algebra"]), 3).unwrap(), "401.01.Algebra");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "PJ.Tesis"]), 3).unwrap(), "401.PJ.Tesis");
    }

    #[test]
    fn review_shortcut_name_adds_a_slot_for_each_extra_level() {
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math"]), 4).unwrap(), "401.00.00.Math");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "01.Algebra"]), 4).unwrap(), "401.01.00.Algebra");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "01.Algebra", "01.Sub"]), 4).unwrap(), "401.01.01.Sub");
    }

    #[test]
    fn review_shortcut_name_rejects_invalid_owners() {
        assert!(review_shortcut_name(&parts(&[]), 3).is_err());
        assert!(review_shortcut_name(&parts(&["a", "b", "c", "d"]), 3).is_err());
        assert!(review_shortcut_name(&parts(&["", "01.Math"]), 3).is_err());
    }
}
