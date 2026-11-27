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

mod codex {
    use super::*;
    use crate::services::ai_clients::codex::{config_dir as codex_dir, connect_in, disconnect_in, entry_command, is_connected_in, with_entry, without_entry};

    const TABLE: &str = "[mcp_servers.ant-system]\ncommand = \"C:\\\\Apps\\\\FolderManager.exe\"\nargs = [\"--mcp\"]\n";

    #[test]
    fn connect_appends_the_table_and_keeps_the_rest_byte_identical() {
        let before = "model = \"gpt-5\"  # keep\n\n[mcp_servers.other]\ncommand = \"x\"\n";
        assert_eq!(with_entry(before, EXE).unwrap(), format!("{before}\n{TABLE}"));
        assert_eq!(with_entry("", EXE).unwrap(), TABLE);
        assert_eq!(with_entry("a = 1", EXE).unwrap(), format!("a = 1\n\n{TABLE}"));
    }

    #[test]
    fn connect_keeps_crlf_files_crlf() {
        assert_eq!(with_entry("a = 1\r\n", EXE).unwrap(), format!("a = 1\r\n\r\n{}", TABLE.replace('\n', "\r\n")));
    }

    #[test]
    fn disconnect_undoes_connect_and_removes_only_ours() {
        for before in ["", "a = 1\n", "a = 1\n\n[mcp_servers.other]\ncommand = \"x\"\n", "[mcp_servers.ant-system-2]\ncommand = \"y\"\n"] {
            assert_eq!(without_entry(&with_entry(before, EXE).unwrap()).unwrap(), before);
        }
        let middle = "[mcp_servers.ant-system]\ncommand = \"old\"\n[mcp_servers.ant-system.env]\nK = \"v\"\n\n[next]\nz = 1\n";
        assert_eq!(without_entry(middle).unwrap(), "\n[next]\nz = 1\n");
        assert_eq!(without_entry("a = 1\n").unwrap(), "a = 1\n");
    }

    #[test]
    fn connect_replaces_an_existing_entry_instead_of_duplicating_it() {
        let after = with_entry("[mcp_servers.\"ant-system\"]\ncommand = 'old.exe'\n\n[tui]\nx = 1\n", EXE).unwrap();
        assert_eq!(after, format!("\n[tui]\nx = 1\n\n{TABLE}"));
        assert_eq!(after.matches("ant-system").count(), 1);
    }

    #[test]
    fn the_command_is_read_from_either_string_form() {
        assert_eq!(entry_command(TABLE).as_deref(), Some(EXE));
        assert_eq!(entry_command("[ mcp_servers . 'ant-system' ]\ncommand = 'C:\\Apps\\fm.exe'\n").as_deref(), Some(r"C:\Apps\fm.exe"));
        assert_eq!(entry_command("[mcp_servers.other]\ncommand = \"x\"\n"), None);
        assert_eq!(entry_command("[mcp_servers.ant-system]\nargs = []\n"), None);
    }

    #[test]
    fn inline_and_dotted_forms_are_refused() {
        for text in ["mcp_servers = { other = { command = \"x\" } }\n", "mcp_servers.other.command = \"x\"\n", "[mcp_servers]\nant-system = { command = \"x\" }\n"] {
            assert!(with_entry(text, EXE).is_err(), "{text}");
            assert!(without_entry(text).is_err(), "{text}");
        }
        assert!(with_entry("[mcp_servers]\nother = { command = \"x\" }\n", EXE).is_ok());
    }

    #[test]
    fn files_are_created_changed_and_checked() {
        let dir = TempDir::new("ai-codex");
        let file = dir.path().join("config.toml");
        disconnect_in(dir.path()).unwrap();
        assert!(!file.exists());
        connect_in(dir.path(), EXE).unwrap();
        assert!(is_connected_in(dir.path(), EXE).unwrap());
        assert!(!is_connected_in(dir.path(), r"C:\Other\fm.exe").unwrap());
        assert!(!dir.path().join("config.toml.bak").exists());
        fs::write(&file, "mcp_servers.x.command = \"y\"\n").unwrap();
        assert!(connect_in(dir.path(), EXE).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "mcp_servers.x.command = \"y\"\n");
        fs::write(&file, "a = 1\n").unwrap();
        connect_in(dir.path(), EXE).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("config.toml.bak")).unwrap(), "a = 1\n");
        disconnect_in(dir.path()).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "a = 1\n");
    }

    #[test]
    fn codex_home_wins_over_the_user_profile() {
        let home = TempDir::new("ai-codex-home");
        let custom = TempDir::new("ai-codex-custom");
        assert_eq!(codex_dir(None, Some(home.path())), Some(home.path().join(".codex")));
        assert_eq!(codex_dir(Some(custom.path()), Some(home.path())), Some(custom.path().to_path_buf()));
    }
}

mod opencode {
    use super::*;
    use crate::services::ai_clients::opencode::{config_dir as opencode_dir, connect_in, disconnect_in, is_connected_in};

    fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
        dir.path().join(name)
    }

    #[test]
    fn connect_creates_the_file_with_a_local_entry() {
        let dir = TempDir::new("ai-oc-new");
        connect_in(dir.path(), EXE).unwrap();
        let config: Value = serde_json::from_str(&fs::read_to_string(path(&dir, "opencode.json")).unwrap()).unwrap();
        assert_eq!(config["mcp"]["ant-system"], json!({ "type": "local", "command": [EXE, "--mcp"], "enabled": true }));
        assert!(is_connected_in(dir.path(), EXE).unwrap());
        assert!(!is_connected_in(dir.path(), r"C:\Other\fm.exe").unwrap());
    }

    #[test]
    fn connect_and_disconnect_touch_only_our_entry() {
        let dir = TempDir::new("ai-oc-merge");
        fs::write(path(&dir, "opencode.json"), r#"{"theme":"dark","mcp":{"other":{"type":"remote","url":"u"}}}"#).unwrap();
        connect_in(dir.path(), EXE).unwrap();
        disconnect_in(dir.path()).unwrap();
        let config: Value = serde_json::from_str(&fs::read_to_string(path(&dir, "opencode.json")).unwrap()).unwrap();
        assert_eq!(config, json!({ "theme": "dark", "mcp": { "other": { "type": "remote", "url": "u" } } }));
    }

    #[test]
    fn a_config_with_comments_or_a_bad_shape_is_refused_and_untouched() {
        let dir = TempDir::new("ai-oc-bad");
        let jsonc = "{\n  // my theme\n  \"theme\": \"dark\"\n}\n";
        fs::write(path(&dir, "opencode.jsonc"), jsonc).unwrap();
        assert!(connect_in(dir.path(), EXE).is_err());
        assert!(disconnect_in(dir.path()).is_err());
        assert!(is_connected_in(dir.path(), EXE).is_err());
        assert_eq!(fs::read_to_string(path(&dir, "opencode.jsonc")).unwrap(), jsonc);
        assert!(!path(&dir, "opencode.json").exists());
        fs::write(path(&dir, "opencode.jsonc"), r#"{"mcp":[1]}"#).unwrap();
        assert!(connect_in(dir.path(), EXE).is_err());
    }

    #[test]
    fn a_plain_jsonc_file_is_used_when_there_is_no_json() {
        let dir = TempDir::new("ai-oc-jsonc");
        fs::write(path(&dir, "opencode.jsonc"), r#"{"theme":"dark"}"#).unwrap();
        connect_in(dir.path(), EXE).unwrap();
        assert!(fs::read_to_string(path(&dir, "opencode.jsonc")).unwrap().contains("ant-system"));
        assert!(!path(&dir, "opencode.json").exists());
    }

    #[test]
    fn disconnect_without_a_file_changes_nothing() {
        let dir = TempDir::new("ai-oc-none");
        disconnect_in(dir.path()).unwrap();
        assert!(!path(&dir, "opencode.json").exists());
    }

    #[test]
    fn xdg_config_home_wins_over_the_user_profile() {
        let home = TempDir::new("ai-oc-home");
        let xdg = TempDir::new("ai-oc-xdg");
        assert_eq!(opencode_dir(None, Some(home.path())), Some(home.path().join(".config").join("opencode")));
        assert_eq!(opencode_dir(Some(xdg.path()), Some(home.path())), Some(xdg.path().join("opencode")));
    }
}
