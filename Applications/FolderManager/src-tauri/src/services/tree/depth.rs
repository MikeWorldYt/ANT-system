// The tree depth limit is a per-storage setting in ant.db; every walk of the tree reads it here.
use std::path::Path;

use crate::config::{DEPTH_CEILING, MAX_DEPTH};
use crate::error::R;
use crate::storage::db::Store;

pub const DEPTH_KEY: &str = "treeDepth";

#[derive(Debug, PartialEq, Eq)]
pub struct DepthSetting {
    pub limit: usize,
    pub warning: Option<String>,
}

// Values under the default are refused: the level rules (data flag on level 3, workboards below the root) need three levels.
pub fn parse(value: &str) -> Option<usize> {
    value.parse().ok().filter(|n| (MAX_DEPTH..=DEPTH_CEILING).contains(n) && n.to_string() == value)
}

// A stored value the app would not write falls back to the default and says so, instead of being dropped silently.
pub fn read(root: &Path) -> R<DepthSetting> {
    let stored = match Store::open_existing(root)? {
        Some(store) => store.settings()?.remove(DEPTH_KEY),
        None => None,
    };
    Ok(match stored.as_deref().map(|value| (value, parse(value))) {
        None => DepthSetting { limit: MAX_DEPTH, warning: None },
        Some((_, Some(limit))) => DepthSetting { limit, warning: None },
        Some((value, None)) => DepthSetting {
            limit: MAX_DEPTH,
            warning: Some(format!("The saved tree depth \"{value}\" is not between {MAX_DEPTH} and {DEPTH_CEILING}, so the tree uses {MAX_DEPTH} levels.")),
        },
    })
}

pub fn limit(root: &Path) -> R<usize> {
    read(root).map(|setting| setting.limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn stored(value: &str) -> TempDir {
        let dir = TempDir::new("depth-setting");
        Store::open_or_create(dir.path()).unwrap().set_setting(DEPTH_KEY, value).unwrap();
        dir
    }

    #[test]
    fn the_default_is_three_without_a_db_or_a_stored_value() {
        let dir = TempDir::new("depth-default");
        assert_eq!(read(dir.path()).unwrap(), DepthSetting { limit: 3, warning: None });
        Store::open_or_create(dir.path()).unwrap();
        assert_eq!(read(dir.path()).unwrap(), DepthSetting { limit: 3, warning: None });
    }

    #[test]
    fn only_three_to_six_written_plainly_parse() {
        for (value, expected) in [("3", Some(3)), ("4", Some(4)), ("6", Some(6)), ("7", None), ("2", None), ("04", None), ("+4", None), ("", None), ("x", None)] {
            assert_eq!(parse(value), expected, "{value}");
        }
    }

    #[test]
    fn a_stored_value_is_honored() {
        assert_eq!(limit(stored("5").path()).unwrap(), 5);
    }

    #[test]
    fn an_invalid_stored_value_falls_back_to_three_with_a_warning() {
        for value in ["9", "deep", "1"] {
            let setting = read(stored(value).path()).unwrap();
            assert_eq!(setting.limit, 3, "{value}");
            let warning = setting.warning.expect("a fallback is reported");
            assert!(warning.contains(value) && warning.contains("3 levels"), "{warning}");
        }
    }
}
