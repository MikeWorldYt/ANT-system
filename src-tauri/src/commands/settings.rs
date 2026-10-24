// User preferences kept in ant.db, and the links the settings window opens in the default browser or mail app.
use std::collections::{HashMap, HashSet};
use std::process::Command;

use serde::Deserialize;
use tauri::AppHandle;

use crate::commands::run_locked;
use crate::storage::db::Store;
use crate::error::{FmError, R};
use crate::updater::CURRENT_VERSION;

const ISSUES_URL: &str = "https://github.com/MikeWorldYt/ANT-system/issues/new";
const RELEASES_URL: &str = "https://github.com/MikeWorldYt/ANT-system/releases/tag";
const DOCS_URL: &str = "https://ant.mikeworld.dev";
const MAIL_ADDRESS: &str = "info@mikeworld.dev";
// A new key is added here before the UI can write it: on/off switches, the launch choice and the shortcut overrides.
const BOOLEAN_SETTINGS: [&str; 6] = ["pinFavorites", "hideDone", "confirmDelete", "autoUpdate", "debugLogging", "backupAuto"];
const ON_LAUNCH_VALUES: [&str; 2] = ["continue", "root"];
const START_VIEW_VALUES: [&str; 3] = ["continue", "tree", "columns"];
const BACKUP_FREQUENCIES: [&str; 3] = ["day", "week", "month"];
const MAX_BACKUP_FOLDER_CHARS: usize = 400;
const SHORTCUT_ACTIONS: [&str; 6] = ["focusSearch", "editPath", "newNode", "newWorkboard", "openSettings", "goUp"];
// Keys the folder views own, so a shortcut can never take them: (code, mod, alt).
const FIXED_KEYS: [(&str, bool, bool); 2] = [("Delete", false, false), ("Enter", false, false)];

#[derive(Deserialize)]
struct Combo {
    code: String,
    #[serde(default)]
    r#mod: bool,
    #[serde(default)]
    alt: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Link {
    Issues,
    Mail,
    Changelog,
    Docs,
}

// Only fixed addresses are ever opened, so nothing typed by a user reaches the command line.
fn link_target(link: Link, version: &str) -> String {
    match link {
        Link::Issues => format!("{ISSUES_URL}?title=&body=Version%3A%20{version}%0A%0A"),
        Link::Mail => format!("mailto:{MAIL_ADDRESS}?subject=ANT%20System%20Manager%20{version}"),
        Link::Changelog => format!("{RELEASES_URL}/folder-manager-v{version}"),
        Link::Docs => DOCS_URL.to_string(),
    }
}

fn shortcuts_valid(value: &str) -> bool {
    let Ok(combos) = serde_yaml::from_str::<HashMap<String, Combo>>(value) else { return false };
    let mut seen = HashSet::new();
    combos.iter().all(|(action, c)| {
        SHORTCUT_ACTIONS.contains(&action.as_str())
            && !c.code.is_empty()
            && c.code.len() <= 16
            && c.code.chars().all(|ch| ch.is_ascii_alphanumeric())
            && (c.r#mod || c.alt)
            && !FIXED_KEYS.contains(&(c.code.as_str(), c.r#mod, c.alt))
            && seen.insert((c.code.clone(), c.r#mod, c.alt))
    })
}

// Empty means the default folder inside the storage.
pub(crate) fn valid_backup_folder(value: &str) -> bool {
    let path = std::path::Path::new(value);
    value.is_empty()
        || (value.chars().count() <= MAX_BACKUP_FOLDER_CHARS
            && !value.chars().any(char::is_control)
            && path.is_absolute()
            && !path.components().any(|c| c == std::path::Component::ParentDir))
}

fn check(key: &str, value: &str) -> R<()> {
    let valid = match key {
        "onLaunch" => ON_LAUNCH_VALUES.contains(&value),
        "startView" => START_VIEW_VALUES.contains(&value),
        "shortcuts" => shortcuts_valid(value),
        "backupFrequency" => BACKUP_FREQUENCIES.contains(&value),
        "backupFolder" => valid_backup_folder(value),
        _ => BOOLEAN_SETTINGS.contains(&key) && matches!(value, "true" | "false"),
    };
    if !valid {
        return Err(FmError::Internal { message: format!("Unknown setting {key}={value}") });
    }
    Ok(())
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> R<HashMap<String, String>> {
    run_locked(app, |root| match Store::open_existing(root)? {
        Some(store) => store.settings(),
        None => Ok(HashMap::new()),
    })
    .await
}

#[tauri::command]
pub async fn set_setting(app: AppHandle, key: String, value: String) -> R<()> {
    check(&key, &value)?;
    run_locked(app, move |root| Store::open_or_create(root)?.set_setting(&key, &value)).await
}

#[tauri::command]
pub async fn reset_settings(app: AppHandle) -> R<()> {
    run_locked(app, |root| match Store::open_existing(root)? {
        Some(mut store) => store.reset_settings(),
        None => Ok(()),
    })
    .await
}

#[cfg(windows)]
fn launcher() -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("rundll32");
    cmd.arg("url.dll,FileProtocolHandler").creation_flags(crate::fs_safe::CREATE_NO_WINDOW);
    cmd
}

#[cfg(target_os = "macos")]
fn launcher() -> Command {
    Command::new("open")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn launcher() -> Command {
    Command::new("xdg-open")
}

#[tauri::command]
pub fn open_link(link: Link) -> R<()> {
    launcher()
        .arg(link_target(link, CURRENT_VERSION))
        .spawn()
        .map(|_| ())
        .map_err(|e| FmError::Internal { message: format!("Could not open the link: {e}") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_switches_with_boolean_values_are_accepted() {
        assert!(check("pinFavorites", "true").is_ok());
        assert!(check("pinFavorites", "yes").is_err());
        assert!(check("other", "true").is_err());
        assert!(check("debugLogging", "false").is_ok());
    }

    #[test]
    fn the_launch_choice_and_shortcut_overrides_are_validated() {
        assert!(check("onLaunch", "root").is_ok());
        assert!(check("onLaunch", "home").is_err());
        assert!(check("startView", "columns").is_ok());
        assert!(check("startView", "list").is_err());
        assert!(check("shortcuts", "{}").is_ok());
        assert!(check("shortcuts", r#"{"goUp": {"code": "ArrowUp", "mod": false, "alt": true}}"#).is_ok());
        assert!(check("shortcuts", r#"{"goUp": {"code": "ArrowUp"}}"#).is_err());
        assert!(check("shortcuts", r#"{"nope": {"code": "KeyJ", "mod": true}}"#).is_err());
        assert!(check("shortcuts", r#"{"goUp": {"code": "Delete"}}"#).is_err());
        let twice = r#"{"goUp": {"code": "KeyJ", "mod": true}, "editPath": {"code": "KeyJ", "mod": true}}"#;
        assert!(check("shortcuts", twice).is_err());
    }

    #[test]
    fn backup_settings_are_validated() {
        let abs = if cfg!(windows) { "C:\\Backups\\ANT" } else { "/backups/ant" };
        assert!(check("backupAuto", "true").is_ok());
        assert!(check("backupAuto", "1").is_err());
        assert!(check("backupFrequency", "month").is_ok());
        assert!(check("backupFrequency", "hour").is_err());
        assert!(check("backupFolder", "").is_ok());
        assert!(check("backupFolder", abs).is_ok());
        assert!(check("backupFolder", "relative/dir").is_err());
        assert!(check("backupFolder", &format!("{abs}/../x")).is_err());
        assert!(check("backupFolder", &format!("{abs}\nx")).is_err());
        assert!(check("backupFolder", &format!("{abs}/{}", "a".repeat(400))).is_err());
        assert!(check("backupLast", "123").is_err());
    }

    #[test]
    fn resetting_drops_only_the_settings() {
        let dir = crate::test_support::TempDir::new("reset-settings");
        let mut store = Store::open_or_create(dir.path()).unwrap();
        store.set_setting("hideDone", "true").unwrap();
        store.reset_settings().unwrap();
        assert!(store.settings().unwrap().is_empty());
        assert!(Store::open_existing(dir.path()).unwrap().is_some());
    }

    #[test]
    fn links_carry_the_version_and_fixed_addresses() {
        assert!(link_target(Link::Issues, "2.0.1").starts_with("https://github.com/MikeWorldYt/ANT-system/issues/new?"));
        assert!(link_target(Link::Issues, "2.0.1").contains("Version%3A%202.0.1"));
        assert!(link_target(Link::Mail, "2.0.1").starts_with("mailto:info@mikeworld.dev?"));
        assert!(link_target(Link::Changelog, "2.1.5").ends_with("/releases/tag/folder-manager-v2.1.5"));
        assert_eq!(link_target(Link::Docs, "2.1.5"), "https://ant.mikeworld.dev");
    }
}
