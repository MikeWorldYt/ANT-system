// Fixed folder names, Tree.yaml layout rules and update endpoints shared by the core modules.
use serde::{Deserialize, Serialize};

pub const VERSION_URL: &str =
    "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Applications/FolderManager/version.txt";
pub const EXE_DOWNLOAD_URL: &str =
    "https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe";
pub const UPDATER_SCRIPT: &str = "FM_Updater.bat";

pub const SETTINGS_FOLDER: &str = "999.Settings";
pub const CORE_FOLDER: &str = "00.Core";
pub const YAML_FILENAME: &str = "Tree.yaml";
pub const REVIEW_FOLDER: &str = "998.Review";
pub const TRASH_DIR: &str = ".ant-trash";
pub const DATA_DIR: &str = ".ant";
pub const DB_FILENAME: &str = "ant.db";
pub const INFO_FILENAME: &str = "info.json";
pub const MAX_DEPTH: usize = 3;

pub const LOCKED_TYPES: &[&str] = &["REV", "RES", "TPL", "CFG"];
pub const LINKED_PREFIXES: &[&str] = &["90", "98"];
pub const WKB_IDENTIFIERS: &[&str] = &["PJ.", "PR.", "HW."];
pub const SPECIAL_TYPES: &[(&str, &str)] = &[("90.", "RES"), ("97.", "TPL"), ("98.", "REV")];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TypeBase {
    Node,
    Wkb,
    Portf,
    Rev,
    Res,
    Tpl,
    Cfg,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EditableType {
    Node,
    Wkb,
    Portf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WkbStatus {
    InProgress,
    ForReview,
    Hold,
    Done,
}

// Storage-root folders the app owns; they are never nodes, whatever their case.
pub fn is_hidden_dir(name: &str) -> bool {
    [TRASH_DIR, DATA_DIR].iter().any(|d| name.eq_ignore_ascii_case(d))
}
