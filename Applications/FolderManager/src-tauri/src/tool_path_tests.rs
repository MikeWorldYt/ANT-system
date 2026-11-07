// Keeps Windows tools from being started by a bare name, which Rust resolves from the app folder first.
use std::fs;
use std::path::{Path, PathBuf};

const TOOLS: [&str; 7] = ["explorer", "rundll32", "attrib", "powershell", "curl", "schtasks", "cmd"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("Could not list {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|e| panic!("Could not list {}: {e}", dir.display())).path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

// The owner keeps the launch of FM_Updater.bat as it is, so that one `cmd` call is the only exemption.
fn exempt(file: &Path, line: &str) -> bool {
    file.ends_with("platform/windows/updater.rs") && line.contains(r#"Command::new("cmd")"#)
}

#[test]
fn no_windows_tool_starts_from_a_bare_name() {
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found");
    let mut found = Vec::new();
    for file in files.iter().filter(|f| !f.ends_with("platform/windows/system.rs") && !f.to_string_lossy().ends_with("_tests.rs")) {
        let text = fs::read_to_string(file).unwrap_or_else(|e| panic!("Could not read {}: {e}", file.display()));
        let lines: Vec<&str> = text.lines().take_while(|l| !l.contains("#[cfg(test)]")).collect();
        for (n, line) in lines.iter().enumerate() {
            let other_os = lines[n.saturating_sub(3)..n].iter().any(|l| l.contains("not(windows)") || l.contains("unix") || l.contains("macos"));
            let bare = TOOLS.iter().any(|t| [format!("Command::new(\"{t}\")"), format!("Command::new(\"{t}.exe\")"), format!("from(\"{t}.exe\")"), format!("\"{t}.exe\".into()")].iter().any(|p| line.contains(p)));
            if bare && !other_os && !exempt(file, line) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "bare Windows tool names:\n{}", found.join("\n"));
}
