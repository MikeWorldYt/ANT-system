// Keeps domain at the bottom of the layers: it may use only itself, std and external crates.
use std::fs;
use std::path::{Path, PathBuf};

const UPPER_LAYERS: [&str; 5] = ["crate::app", "crate::commands", "crate::infra", "crate::platform", "crate::services"];

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

#[test]
fn domain_does_not_import_upper_layers() {
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("domain"), &mut files);
    assert!(!files.is_empty(), "no domain sources found");
    let mut found = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap_or_else(|e| panic!("Could not read {}: {e}", file.display()));
        for (n, line) in text.lines().enumerate() {
            if UPPER_LAYERS.iter().any(|layer| line.contains(layer)) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "domain imports an upper layer:\n{}", found.join("\n"));
}
