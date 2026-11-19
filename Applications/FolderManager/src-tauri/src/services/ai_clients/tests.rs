use std::fs;

use serde_json::{json, Value};

use super::claude_desktop::{config_dir, connect_in, disconnect_in, is_connected_in, write_config};
use crate::test_support::TempDir;

const EXE: &str = r"C:\Apps\FolderManager.exe";
const FILE: &str = "claude_desktop_config.json";

fn read(dir: &TempDir) -> Value {
    serde_json::from_str(&fs::read_to_string(dir.path().join(FILE)).unwrap()).unwrap()
}

fn write(dir: &TempDir, text: &str) {
    fs::write(dir.path().join(FILE), text).unwrap();
}

#[test]
fn connect_creates_the_file_when_missing() {
    let dir = TempDir::new("ai-new");
    connect_in(dir.path(), EXE).unwrap();
    assert_eq!(read(&dir)["mcpServers"]["ant-system"], json!({ "command": EXE, "args": ["--mcp"] }));
}

#[test]
fn connect_keeps_other_servers_and_keys() {
    let dir = TempDir::new("ai-merge");
    write(&dir, r#"{"theme":"dark","mcpServers":{"other":{"command":"x"}}}"#);
    connect_in(dir.path(), EXE).unwrap();
    let config = read(&dir);
    assert_eq!(config["theme"], "dark");
    assert_eq!(config["mcpServers"]["other"], json!({ "command": "x" }));
    assert_eq!(config["mcpServers"]["ant-system"]["command"], EXE);
}

#[test]
fn invalid_json_is_refused_and_untouched() {
    let dir = TempDir::new("ai-bad");
    write(&dir, "{ not json");
    assert!(connect_in(dir.path(), EXE).is_err());
    assert_eq!(fs::read_to_string(dir.path().join(FILE)).unwrap(), "{ not json");
    write(&dir, "[1]");
    assert!(connect_in(dir.path(), EXE).is_err());
    assert!(disconnect_in(dir.path()).is_err());
}

#[test]
fn disconnect_removes_only_ours() {
    let dir = TempDir::new("ai-off");
    write(&dir, r#"{"mcpServers":{"other":{"command":"x"}}}"#);
    connect_in(dir.path(), EXE).unwrap();
    disconnect_in(dir.path()).unwrap();
    assert_eq!(read(&dir), json!({ "mcpServers": { "other": { "command": "x" } } }));
}

#[test]
fn disconnect_without_a_file_changes_nothing() {
    let dir = TempDir::new("ai-none");
    disconnect_in(dir.path()).unwrap();
    assert!(!dir.path().join(FILE).exists());
}

#[test]
fn connected_needs_a_matching_command() {
    let dir = TempDir::new("ai-conn");
    assert!(!is_connected_in(dir.path(), EXE).unwrap());
    connect_in(dir.path(), EXE).unwrap();
    assert!(is_connected_in(dir.path(), EXE).unwrap());
    assert!(!is_connected_in(dir.path(), r"C:\Other\fm.exe").unwrap());
    #[cfg(windows)]
    assert!(is_connected_in(dir.path(), &EXE.to_lowercase()).unwrap());
}

#[test]
fn packaged_dir_wins_over_roaming() {
    let local = TempDir::new("ai-local");
    let roaming = TempDir::new("ai-roam");
    assert_eq!(config_dir(Some(local.path()), Some(roaming.path())), Some(roaming.path().join("Claude")));
    let packaged = local.path().join("Packages").join("Claude_abc").join("LocalCache").join("Roaming").join("Claude");
    fs::create_dir_all(&packaged).unwrap();
    assert_eq!(config_dir(Some(local.path()), Some(roaming.path())), Some(packaged));
    let data = local.path().join("Claude-Data").join("claude_abc");
    fs::create_dir_all(&data).unwrap();
    assert_eq!(config_dir(Some(local.path()), Some(roaming.path())), Some(data));
}

#[test]
fn a_write_keeps_the_key_order_and_backs_up_the_previous_file() {
    let dir = TempDir::new("ai-bak");
    let before = r#"{"zeta":1,"mcpServers":{"other":{"command":"x"}},"alpha":2}"#;
    write(&dir, before);
    connect_in(dir.path(), EXE).unwrap();
    let text = fs::read_to_string(dir.path().join(FILE)).unwrap();
    let keys: Vec<usize> = ["\"zeta\"", "\"mcpServers\"", "\"alpha\""].iter().map(|k| text.find(k).unwrap()).collect();
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "{text}");
    assert_eq!(fs::read_to_string(dir.path().join(format!("{FILE}.bak"))).unwrap(), before);
    let mut left: Vec<String> = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    left.sort();
    assert_eq!(left, [FILE.to_string(), format!("{FILE}.bak")]);
}

#[test]
fn a_missing_or_invalid_file_gets_no_backup() {
    let dir = TempDir::new("ai-nobak");
    connect_in(dir.path(), EXE).unwrap();
    assert!(!dir.path().join(format!("{FILE}.bak")).exists());
    let bad = TempDir::new("ai-nobak-bad");
    write(&bad, "{ not json");
    assert!(connect_in(bad.path(), EXE).is_err());
    assert!(!bad.path().join(format!("{FILE}.bak")).exists());
}

#[test]
fn a_file_changed_since_the_read_is_not_overwritten() {
    let dir = TempDir::new("ai-race");
    write(&dir, r#"{"theme":"light"}"#);
    let mut config = serde_json::Map::new();
    config.insert("theme".into(), json!("dark"));
    assert!(write_config(&dir.path().join(FILE), Some(r#"{"theme":"old"}"#), &config).is_err());
    assert_eq!(fs::read_to_string(dir.path().join(FILE)).unwrap(), r#"{"theme":"light"}"#);
    assert!(write_config(&dir.path().join(FILE), None, &config).is_err());
    write_config(&dir.path().join(FILE), Some(r#"{"theme":"light"}"#), &config).unwrap();
    assert_eq!(read(&dir), json!({ "theme": "dark" }));
}
