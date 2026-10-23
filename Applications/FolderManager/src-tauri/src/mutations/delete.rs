// Reorder nodes by swapping their numeric ids, and delete folders (via .ant-trash and the Recycle Bin) or ghost records.
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::recycle::{PendingRecycle, IN_FLIGHT};
use super::{finish, load_synced, locate, persist, split, Mutation, Target};
use crate::config::TRASH_DIR;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, redirects, Journal, StorageLock, MAX_NAME_UNITS};
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, delete_key, is_locked_type};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreview {
    pub ghost: bool,
    pub files: usize,
    pub dirs: usize,
}

pub fn inspect_delete(root: &Path, rel: &[String]) -> R<DeletePreview> {
    let (data, _) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_delete(rel, &target)?;
    if target.ghost {
        return Ok(DeletePreview { ghost: true, files: 0, dirs: 0 });
    }
    let (files, dirs) = count_contents(&target.abs)?;
    Ok(DeletePreview { ghost: false, files, dirs })
}

// The caller runs `recycle_staged` on the returned pending folder after releasing the storage lock.
pub fn delete_node(
    root: &Path,
    rel: &[String],
    expect_ghost: bool,
    confirm_non_empty: bool,
) -> R<(Mutation, Option<PendingRecycle>)> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    if target.ghost != expect_ghost {
        return Err(FmError::Stale { rel_parts: rel.to_vec() });
    }
    guard_delete(rel, &target)?;
    let (name, parent) = split(rel)?;
    if target.ghost {
        if !delete_key(&mut data, parent, name) {
            return Err(FmError::NotFound { rel_parts: rel.to_vec() });
        }
        tree_store::save(root, &data)?;
        return Ok((finish(root, &data, synced, None, Vec::new()), None));
    }
    let (files, dirs) = count_contents(&target.abs)?;
    if files + dirs > 0 && !confirm_non_empty {
        return Err(FmError::NeedsConfirmation { files, dirs });
    }

    let mut journal = Journal::new();
    let staged = stage_in_trash(root, &mut journal, &target.abs, name)?;
    delete_key(&mut data, parent, name);
    persist(root, &data, journal, "delete")?;
    IN_FLIGHT.lock().unwrap_or_else(|p| p.into_inner()).push(staged.clone());
    let pending = PendingRecycle { root: root.to_path_buf(), staged, name: name.clone() };
    Ok((finish(root, &data, synced, None, Vec::new()), Some(pending)))
}

// A locked ghost is only a stale record, so it may go; a locked real folder may not.
fn guard_delete(rel: &[String], target: &Target) -> R<()> {
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if !target.ghost && is_locked_type(&target.node_type) {
        return Err(FmError::Locked { rel_parts: rel.to_vec(), node_type: target.node_type.clone() });
    }
    Ok(())
}

// Links and junctions are counted but never followed; cloud placeholders are real folders and are entered.
fn count_contents(dir: &Path) -> R<(usize, usize)> {
    let (mut files, mut dirs) = (0, 0);
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current).map_err(|e| io_err("list folder", &current, &e))?;
        for entry in entries {
            let path = entry.map_err(|e| io_err("list folder", &current, &e))?.path();
            let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e))?;
            if meta.is_dir() {
                dirs += 1;
                if !redirects(&path, &meta)? {
                    pending.push(path);
                }
            } else {
                files += 1;
            }
        }
    }
    Ok((files, dirs))
}

fn stage_in_trash(root: &Path, journal: &mut Journal, abs: &Path, name: &str) -> R<PathBuf> {
    let trash = root.join(TRASH_DIR);
    match fs::symlink_metadata(&trash) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => journal.mkdir(&trash)?,
        Err(e) => return Err(io_err("inspect path", &trash, &e)),
        Ok(meta) if !meta.is_dir() || redirects(&trash, &meta)? => {
            return Err(FmError::InvalidPath {
                rel_parts: vec![TRASH_DIR.to_string()],
                reason: "not a plain folder".to_string(),
            });
        }
        Ok(_) => {}
    }
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let stamp = base36(millis);
    let free = (0..100)
        .map(|n| trash.join(trash_name(&stamp, n, name)))
        .find(|p| fs::symlink_metadata(p).is_err());
    let Some(staged) = free else {
        let cause = FmError::AlreadyExists { name: format!("{TRASH_DIR}/{stamp}-{name}") };
        return Err(std::mem::take(journal).abort("delete", cause));
    };
    if let Err(e) = journal.rename(abs, &staged) {
        return Err(std::mem::take(journal).abort("delete", e));
    }
    Ok(staged)
}

fn base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8_lossy(&out).into_owned()
}

// The prefix makes the name longer, so a name already at the limit is cut to stay a valid folder name.
pub(super) fn trash_name(stamp: &str, n: usize, name: &str) -> String {
    let prefix = if n == 0 { format!("{stamp}-") } else { format!("{stamp}-{n}-") };
    let room = MAX_NAME_UNITS.saturating_sub(prefix.len());
    let mut kept = String::new();
    let mut units = 0;
    for c in name.chars() {
        units += c.len_utf16();
        if units > room {
            break;
        }
        kept.push(c);
    }
    format!("{prefix}{}", kept.trim_end_matches(['.', ' ']))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_safe::validate_component;
    use crate::test_support::*;
    use crate::tree::tree_store::get_node;

    #[test]
    fn delete_ghost_only_touches_yaml_and_checks_expectation() {
        let dir = work("delete-ghost");
        let ghost = rel(&["100.WORK", "05.Ghost"]);
        let stale = delete_with(dir.path(), &ghost, false, false, ok_recycle);
        assert!(matches!(stale, Err(FmError::Stale { .. })));
        let real = delete_with(dir.path(), &rel(&["100.WORK", "02.B"]), true, false, ok_recycle);
        assert!(matches!(real, Err(FmError::Stale { .. })));
        assert_eq!(inspect_delete(dir.path(), &ghost).unwrap(), DeletePreview { ghost: true, files: 0, dirs: 0 });
        let m = delete_with(dir.path(), &ghost, true, false, |_| panic!("no disk delete")).unwrap();
        assert!(m.select.is_none());
        assert!(get_node(&yaml(dir.path()), &ghost).is_none());
        assert!(dir.path().join("100.WORK/02.B").is_dir());
    }

    #[test]
    fn delete_empty_folder_goes_through_the_trash() {
        let dir = work("delete-empty");
        let p = rel(&["100.WORK", "02.B"]);
        assert_eq!(inspect_delete(dir.path(), &p).unwrap(), DeletePreview { ghost: false, files: 0, dirs: 0 });
        let m = delete_with(dir.path(), &p, false, false, ok_recycle).unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        assert!(!dir.path().join("100.WORK/02.B").exists());
        assert!(!dir.path().join(TRASH_DIR).exists());
        assert!(get_node(&yaml(dir.path()), &p).is_none());
    }

    #[test]
    fn delete_non_empty_needs_confirmation_and_keeps_staged_copy_on_recycle_failure() {
        let dir = work("delete-full");
        fs::write(dir.path().join("100.WORK/01.A/01.Keep/notes.txt"), "keep me").unwrap();
        let p = rel(&["100.WORK", "01.A"]);
        assert_eq!(inspect_delete(dir.path(), &p).unwrap(), DeletePreview { ghost: false, files: 1, dirs: 1 });
        let refused = delete_with(dir.path(), &p, false, false, ok_recycle);
        assert!(matches!(refused, Err(FmError::NeedsConfirmation { files: 1, dirs: 1 })));
        assert!(dir.path().join("100.WORK/01.A/01.Keep/notes.txt").is_file());

        let m = delete_with(dir.path(), &p, false, true, |_| Err("bin unavailable".to_string())).unwrap();
        assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
        let staged = names(&dir.path().join(TRASH_DIR));
        assert_eq!(staged.len(), 1);
        assert!(staged[0].ends_with("-01.A"), "{staged:?}");
        let kept = dir.path().join(TRASH_DIR).join(&staged[0]).join("01.Keep/notes.txt");
        assert_eq!(fs::read_to_string(kept).unwrap(), "keep me");
        assert!(get_node(&yaml(dir.path()), &p).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn delete_puts_the_folder_back_when_yaml_save_fails() {
        let dir = work("delete-rollback");
        let _held = break_tree_save(dir.path());
        let p = rel(&["100.WORK", "02.B"]);
        let err = delete_with(dir.path(), &p, false, false, |_| panic!("not staged")).unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert!(dir.path().join("100.WORK/02.B").is_dir());
        assert!(!dir.path().join(TRASH_DIR).exists());
    }

    #[test]
    fn delete_refuses_reserved_and_locked_folders_but_not_locked_ghosts() {
        let dir = storage("delete-refuse", &["100.WORK/98.Rev", "998.Review"], "100.WORK:\n  97.Gone: {type: TPL}\n");
        for reserved in ["999.Settings", "998.Review"] {
            let err = delete_with(dir.path(), &rel(&[reserved]), false, true, ok_recycle);
            assert!(matches!(err, Err(FmError::Reserved { .. })), "{reserved}");
        }
        let locked = delete_with(dir.path(), &rel(&["100.WORK", "98.Rev"]), false, true, ok_recycle);
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        delete_with(dir.path(), &rel(&["100.WORK", "97.Gone"]), true, false, ok_recycle).unwrap();
        assert!(dir.path().join("100.WORK/98.Rev").is_dir() && dir.path().join("999.Settings").is_dir());
    }

    #[test]
    fn delete_refuses_reserved_names_in_any_case() {
        let dir = storage("delete-reserved-case", &["100.WORK"], "998.review: {type: REV}\n999.SETTINGS: {type: CFG}\n");
        for name in ["998.review", "999.SETTINGS"] {
            let err = delete_with(dir.path(), &rel(&[name]), true, false, ok_recycle);
            assert!(matches!(err, Err(FmError::Reserved { .. })), "{name}: {err:?}");
        }
        assert!(get_node(&yaml(dir.path()), &rel(&["998.review"])).is_some());
    }

    #[test]
    fn trash_records_cannot_be_deleted_as_ghosts_and_meta_folders_are_refused() {
        let dir = storage("delete-trash", &["100.WORK/type", ".ant-trash/old"], "100.WORK: {type: NODE-L1}\n.ant-trash: {type: NODE-L1}\n");
        let trash = delete_with(dir.path(), &rel(&[".ant-trash"]), true, false, ok_recycle);
        assert!(matches!(trash, Err(FmError::InvalidPath { .. })), "{trash:?}");
        let meta = delete_with(dir.path(), &rel(&["100.WORK", "type"]), false, true, ok_recycle);
        assert!(matches!(meta, Err(FmError::InvalidName { .. })), "{meta:?}");
        assert!(dir.path().join("100.WORK/type").is_dir() && dir.path().join(".ant-trash/old").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.WORK"]), "NODE-L1");
    }

    #[test]
    fn delete_through_a_wrong_case_parent_only_drops_the_ghost_record() {
        let dir = work("delete-case");
        let mut data = yaml(dir.path());
        data.insert("100.work".into(), serde_yaml::from_str("02.B: {type: NODE-L2}").unwrap());
        crate::tree::tree_store::save(dir.path(), &data).unwrap();
        let p = rel(&["100.work", "02.B"]);
        assert!(matches!(delete_with(dir.path(), &p, false, true, ok_recycle), Err(FmError::Stale { .. })));
        delete_with(dir.path(), &p, true, false, |_| panic!("no disk delete")).unwrap();
        assert!(dir.path().join("100.WORK/02.B").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "02.B"]), "NODE-L2");
    }

    #[test]
    fn trash_names_are_short_and_fit_the_name_limit() {
        assert_eq!(base36(0), "0");
        assert_eq!(base36(36 * 36 + 1), "101");
        assert_eq!(trash_name("abc", 0, "01.A"), "abc-01.A");
        assert_eq!(trash_name("abc", 2, "01.A"), "abc-2-01.A");
        let long = format!("01.{}", "\u{e9}".repeat(252));
        let staged = trash_name("abc", 0, &long);
        assert_eq!(staged.encode_utf16().count(), MAX_NAME_UNITS);
        assert!(validate_component(&staged).is_ok());
        assert_eq!(trash_name("abc", 0, &format!("{}. x", "a".repeat(249))), format!("abc-{}", "a".repeat(249)));
    }

    #[cfg(windows)]
    #[test]
    fn count_contents_does_not_follow_junctions() {
        let dir = work("count-junction");
        let outside = crate::test_support::TempDir::new("count-outside");
        fs::write(outside.path().join("secret.txt"), "x").unwrap();
        let link = dir.path().join("100.WORK/02.B/j");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .is_ok_and(|o| o.status.success());
        if !made {
            eprintln!("skipped: could not create a junction");
            return;
        }
        assert_eq!(count_contents(&dir.path().join("100.WORK/02.B")).unwrap(), (0, 1));
        fs::remove_dir(&link).unwrap();
    }
}
