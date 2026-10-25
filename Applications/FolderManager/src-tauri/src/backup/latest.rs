// The newest backup of the default folder, the only one known when ant.db (and with it the folder setting) is gone.
use std::path::Path;

use serde::Serialize;

use super::{default_dir, entries, fail, restore_from, SUFFIX};
use crate::debug_log::valid_stamp;
use crate::error::R;

#[derive(Debug, PartialEq, Serialize)]
pub struct LatestBackup {
    pub name: String,
    pub label: String,
}

// Stamps are YY-MM-DD-HH-MM, so the label assumes the 2000s.
fn label(name: &str) -> String {
    let stamp = name.split_once(SUFFIX).map_or("", |(s, _)| s);
    match stamp.split('-').collect::<Vec<_>>()[..] {
        [y, m, d, h, min] => format!("20{y}-{m}-{d} {h}:{min}"),
        _ => stamp.to_string(),
    }
}

pub fn newest_default(root: &Path) -> R<Option<LatestBackup>> {
    let newest = entries(&default_dir(root))?.into_iter().next();
    Ok(newest.map(|e| LatestBackup { label: label(&e.name), name: e.name }))
}

pub fn restore_latest(root: &Path, stamp: &str) -> R<()> {
    if !valid_stamp(stamp) {
        return Err(fail(format!("Invalid backup stamp {stamp}")));
    }
    let dir = default_dir(root);
    let Some(latest) = newest_default(root)? else {
        return Err(fail(format!("There is no backup in {}", dir.display())));
    };
    restore_from(root, &dir, &latest.name, stamp)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn the_newest_default_backup_is_found_by_name() {
        let dir = TempDir::new("latest-backup");
        assert_eq!(newest_default(dir.path()).unwrap(), None);
        assert!(restore_latest(dir.path(), "26-10-24-09-05").unwrap_err().to_string().contains("no backup"));
        dir.mkdir("999.Settings/03.Backup");
        let backups = dir.path().join("999.Settings/03.Backup");
        for name in ["26-01-02-03-04_ANT-fm-backup.zip", "26-03-01-10-30_ANT-fm-backup_2.zip", "99-99-99_ANT-fm-backup.zip", "x.zip"] {
            fs::write(backups.join(name), "").unwrap();
        }
        let newest = newest_default(dir.path()).unwrap().unwrap();
        assert_eq!(newest, LatestBackup { name: "26-03-01-10-30_ANT-fm-backup_2.zip".into(), label: "2026-03-01 10:30".into() });
    }
}
