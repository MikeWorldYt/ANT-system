// User preferences kept in ant.db, and the links the settings window opens in the default browser or mail app.
use std::collections::{HashMap, HashSet};
#[cfg(not(windows))]
use std::process::Command;

use serde::Deserialize;
use tauri::AppHandle;

use crate::commands::{run_locked, run_unlocked};
use crate::domain::validation::valid_backup_folder;
use crate::services::backup;
use crate::services::index::review_index;
use crate::infra::storage::db::Store;
use crate::services::tree::depth;
use crate::services::tree::tree_store;
use crate::domain::error::{FmError, R};
use crate::services::updater::CURRENT_VERSION;

const ISSUES_URL: &str = "https://github.com/MikeWorldYt/ANT-system/issues/new";
const RELEASES_URL: &str = "https://github.com/MikeWorldYt/ANT-system/releases/tag";
const DOCS_URL: &str = "https://ant.mikeworld.dev";
const MAIL_ADDRESS: &str = "info@mikeworld.dev";
// A new key is added here before the UI can write it: on/off switches, the launch choice and the shortcut overrides.
const BOOLEAN_SETTINGS: [&str; 12] = [
    "pinFavorites", "hideDone", "hideDirectoryIdentifiers", "confirmDelete", "autoUpdate", "debugLogging", "backupAuto", "watchExternal", "autoOpenColumns", "showFavorites", "showDownloads", "showReview",
];
const ON_LAUNCH_VALUES: [&str; 2] = ["continue", "root"];
const START_VIEW_VALUES: [&str; 6] = ["continue", "tree", "columns", "grid", "list", "cards"];
const TAB_VIEW_VALUES: [&str; 2] = ["last", "default"];
const THEME_VALUES: [&str; 3] = ["system", "light", "dark"];
const ACCENT_VALUES: [&str; 5] = ["green", "blue", "purple", "coral", "gold"];
const TEXT_SIZE_VALUES: [&str; 3] = ["small", "medium", "large"];
const BACKUP_FREQUENCIES: [&str; 3] = ["day", "week", "month"];
const SHORTCUT_ACTIONS: [&str; 11] =
    ["focusSearch", "searchFiles", "editPath", "newNode", "newWorkboard", "openSettings", "goUp", "toggleSidePanel", "moveDown", "toggleFavorite", "editDetails"];
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

fn check(key: &str, value: &str) -> R<()> {
    let valid = match key {
        "onLaunch" => ON_LAUNCH_VALUES.contains(&value),
        "startView" => START_VIEW_VALUES.contains(&value),
        "tabView" => TAB_VIEW_VALUES.contains(&value),
        "theme" => THEME_VALUES.contains(&value),
        "accent" | "iconColor" => ACCENT_VALUES.contains(&value),
        "textSize" => TEXT_SIZE_VALUES.contains(&value),
        "shortcuts" => shortcuts_valid(value),
        "backupFrequency" => BACKUP_FREQUENCIES.contains(&value),
        "backupFolder" => valid_backup_folder(value),
        depth::DEPTH_KEY => depth::parse(value).is_some(),
        _ => BOOLEAN_SETTINGS.contains(&key) && matches!(value, "true" | "false"),
    };
    if !valid {
        return Err(FmError::Internal { message: format!("Unknown setting {key}={value}") });
    }
    Ok(())
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> R<HashMap<String, String>> {
    // A read, so the Settings window opens while a long change holds the lock.
    run_unlocked(app, |_, root| match Store::open_existing(root)? {
        Some(store) => store.settings(),
        None => Ok(HashMap::new()),
    })
    .await
}

#[tauri::command]
pub async fn set_setting(app: AppHandle, key: String, value: String) -> R<()> {
    check(&key, &value)?;
    run_locked(app, move |root| {
        if key == "backupFolder" {
            backup::confirm_folder(&value)?;
        }
        Store::open_or_create(root)?.set_setting(&key, &value)?;
        // Shortcut names carry one slot per level, so a new depth renames every review link.
        if key != depth::DEPTH_KEY {
            return Ok(());
        }
        let failures = review_index::rebuild(root, &tree_store::load(root)?);
        if failures.is_empty() {
            return Ok(());
        }
        Err(FmError::Internal { message: failures.join("; ") })
    })
    .await
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
use crate::platform::windows::url_handler::launcher;

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
        assert!(check("showFavorites", "false").is_ok() && check("showDownloads", "true").is_ok() && check("showReview", "false").is_ok());
        assert!(check("showDownloads", "off").is_err());
    }

    #[test]
    fn the_launch_choice_and_shortcut_overrides_are_validated() {
        assert!(check("onLaunch", "root").is_ok());
        assert!(check("onLaunch", "home").is_err());
        assert!(check("startView", "columns").is_ok());
        assert!(check("startView", "list").is_ok());
        assert!(check("startView", "wall").is_err());
        assert!(check("tabView", "default").is_ok());
        assert!(check("tabView", "wall").is_err());
        assert!(check("theme", "light").is_ok() && check("theme", "sepia").is_err());
        assert!(check("accent", "coral").is_ok() && check("accent", "#fff").is_err());
        assert!(check("textSize", "large").is_ok() && check("textSize", "huge").is_err());
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
        assert!(check("watchExternal", "false").is_ok());
        assert!(check("watchExternal", "yes").is_err());
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
    fn the_tree_depth_is_refused_above_six_and_below_three() {
        for ok in ["3", "4", "6"] {
            assert!(check("treeDepth", ok).is_ok(), "{ok}");
        }
        for bad in ["7", "2", "0", "four", "04"] {
            assert!(check("treeDepth", bad).is_err(), "{bad}");
        }
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
