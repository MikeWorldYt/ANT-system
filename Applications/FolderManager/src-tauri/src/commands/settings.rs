// User preferences kept in ant.db, and the links the settings window opens in the default browser or mail app.
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::process::Command;

use serde::Deserialize;
use tauri::AppHandle;

use crate::commands::run_locked;
use crate::storage::db::Store;
use crate::error::{FmError, R};
use crate::fs_safe::CREATE_NO_WINDOW;
use crate::updater::CURRENT_VERSION;

const ISSUES_URL: &str = "https://github.com/MikeWorldYt/ANT-system/issues/new";
const MAIL_ADDRESS: &str = "info@mikeworld.dev";
// Every setting is an on/off switch for now; a new key is added here before the UI can write it.
const BOOLEAN_SETTINGS: [&str; 1] = ["pinFavorites"];

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Link {
    Issues,
    Mail,
}

// Only fixed addresses are ever opened, so nothing typed by a user reaches the command line.
fn link_target(link: Link, version: &str) -> String {
    match link {
        Link::Issues => format!("{ISSUES_URL}?title=&body=Version%3A%20{version}%0A%0A"),
        Link::Mail => format!("mailto:{MAIL_ADDRESS}?subject=ANT%20System%20Manager%20{version}"),
    }
}

fn check(key: &str, value: &str) -> R<()> {
    if !BOOLEAN_SETTINGS.contains(&key) || !matches!(value, "true" | "false") {
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
pub fn open_link(link: Link) -> R<()> {
    Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(link_target(link, CURRENT_VERSION))
        .creation_flags(CREATE_NO_WINDOW)
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
    }

    #[test]
    fn links_carry_the_version_and_fixed_addresses() {
        assert!(link_target(Link::Issues, "2.0.1").starts_with("https://github.com/MikeWorldYt/ANT-system/issues/new?"));
        assert!(link_target(Link::Issues, "2.0.1").contains("Version%3A%202.0.1"));
        assert!(link_target(Link::Mail, "2.0.1").starts_with("mailto:info@mikeworld.dev?"));
    }
}
