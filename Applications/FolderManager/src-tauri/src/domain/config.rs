// Fixed folder names, tree layout rules and update endpoints shared by the core modules.
use serde::{Deserialize, Serialize};

pub const VERSION_URL: &str =
    "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Applications/FolderManager/version.txt";
pub const EXE_DOWNLOAD_URL: &str =
    "https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe";
pub const UPDATER_SCRIPT: &str = "FM_Updater.bat";
pub const UPDATER_SCRIPT_URL: &str = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/Core/FM_Updater.bat";

// Legacy root folders: still read and recognised, never created.
pub const SETTINGS_FOLDER: &str = "999.Settings";
pub const CORE_FOLDER: &str = "00.Core";
pub const REVIEW_FOLDER: &str = "998.Review";
pub const BACKUP_FOLDER: &str = "backup";
// The Portfolio that a workboard's level gets automatically; the title can be renamed, the 00. id is kept.
pub const PORTFOLIO_FOLDER: &str = "00.Outcomes";
pub const TRASH_DIR: &str = ".ant-trash";
pub const DATA_DIR: &str = ".ant";
pub const DB_FILENAME: &str = "ant.db";
pub const INFO_FILENAME: &str = "info.json";
pub const LINK_FAVORITES_FILENAME: &str = "lnk-favorites.json";
pub const LINK_HIDDEN_FILENAME: &str = "lnk-hidden.json";
pub const FOOTPRINT_FILENAME: &str = "dragpath.json";
// The default tree depth; Settings can raise it per storage up to DEPTH_CEILING.
pub const MAX_DEPTH: usize = 3;
pub const DEPTH_CEILING: usize = 6;

pub const LOCKED_TYPES: &[&str] = &["REV", "RES", "TPL", "CFG"];
pub const DATA_FLAG: &str = "data";
// A hidden folder stays in the storage but is not listed, searched or read.
pub const HIDDEN_FLAG: &str = "hidden";
pub const MAIN_FLAG: &str = "main";
pub const WKB_IDENTIFIERS: &[&str] = &["PJ.", "PR.", "HW.", "CR.", "TK."];
// Support nodes: the word prefix, the numeric one older storages still use, and the type it gives.
pub const SUPPORT_NODES: &[(&str, &str, &str)] = &[("RES.", "90.", "RES"), ("TPL.", "97.", "TPL"), ("REV.", "98.", "REV")];
pub const REVIEW_PREFIX: &str = "REV.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TypeBase {
    Node,
    Wkb,
    Out,
    Rev,
    Res,
    Tpl,
    Cfg,
    Unknown,
}

impl TypeBase {
    const ALL: [Self; 8] = [Self::Node, Self::Wkb, Self::Out, Self::Rev, Self::Res, Self::Tpl, Self::Cfg, Self::Unknown];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Node => "NODE",
            Self::Wkb => "WKB",
            Self::Out => "OUT",
            Self::Rev => "REV",
            Self::Res => "RES",
            Self::Tpl => "TPL",
            Self::Cfg => "CFG",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NewKind {
    Node,
    Workboard,
}

// Portfolio is not offered: each 00.* folder is created and removed with its workboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EditableType {
    Node,
    Wkb,
}

impl EditableType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Node => "NODE",
            Self::Wkb => "WKB",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WkbStatus {
    InProgress,
    ForReview,
    Hold,
    Done,
}

impl WkbStatus {
    const ALL: [Self; 4] = [Self::InProgress, Self::ForReview, Self::Hold, Self::Done];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in-progress",
            Self::ForReview => "for-review",
            Self::Hold => "hold",
            Self::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

// Storage-root folders the app owns; they are never nodes, whatever their case.
pub fn is_hidden_dir(name: &str) -> bool {
    [TRASH_DIR, DATA_DIR].iter().any(|d| name.eq_ignore_ascii_case(d))
}

// Keys that name the same person as `owner`; an existing one is updated instead of adding a second.
pub const OWNER_KEYS: [&str; 3] = ["owner", "holder", "client"];

#[cfg(test)]
mod tests {
    use super::*;

    fn serde_name(value: impl Serialize) -> String {
        serde_yaml::to_string(&value).unwrap().trim().to_string()
    }

    #[test]
    fn string_forms_match_the_serde_names() {
        for t in TypeBase::ALL {
            assert_eq!(t.as_str(), serde_name(t));
            assert_eq!(TypeBase::from_str(t.as_str()), Some(t));
        }
        for s in WkbStatus::ALL {
            assert_eq!(s.as_str(), serde_name(s));
            assert_eq!(WkbStatus::from_str(s.as_str()), Some(s));
        }
        for e in [EditableType::Node, EditableType::Wkb] {
            assert_eq!(e.as_str(), serde_name(e));
        }
        assert_eq!(TypeBase::from_str("node"), None);
        assert_eq!(WkbStatus::from_str("in_progress"), None);
    }
}
