// Reorder nodes by swapping their numeric ids, and delete folders (via .ant-trash and the Recycle Bin) or ghost records.
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::TRASH_DIR;
use crate::edit_actions::{
    child_rel, ensure_free, finish, guard_frozen, load_synced, locate, persist, split, Mutation,
    Target,
};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, is_link, join_rel, same_name, validate_component, Journal};
use crate::names::{editable_part, is_movable_node, list_subdirs, numeric_id};
use crate::tree_node::is_reserved;
use crate::yaml_store::{self, delete_key, get_node, get_node_mut, get_type, is_locked_type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreview {
    pub ghost: bool,
    pub files: usize,
    pub dirs: usize,
}

pub fn move_node(root: &Path, rel: &[String], dir: MoveDirection) -> R<Mutation> {
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "move".to_string() });
    }
    let (name, parent) = split(rel)?;
    if !is_movable_node(name, &target.node_type) {
        return Err(FmError::NotMovable);
    }
    let parent_abs = join_rel(root, parent);
    let mut movable: Vec<(u32, String)> = list_subdirs(&parent_abs)?
        .into_iter()
        .filter(|s| is_movable_node(s, &get_type(&data, &child_rel(parent, s))))
        .filter_map(|s| numeric_id(&s).map(|id| (id, s)))
        .collect();
    movable.sort();
    let idx = movable.iter().position(|(_, s)| s == name).ok_or(FmError::NotMovable)?;
    let swap_idx = match dir {
        MoveDirection::Up => idx.checked_sub(1),
        MoveDirection::Down => Some(idx + 1).filter(|i| *i < movable.len()),
    }
    .ok_or(FmError::NotMovable)?;
    let node_id = movable[idx].0;
    let (swap_id, swap_name) = movable[swap_idx].clone();
    if swap_id == node_id {
        return Err(FmError::NotMovable);
    }

    let node_new = format!("{swap_id:02}.{}", editable_part(name));
    let swap_new = format!("{node_id:02}.{}", editable_part(&swap_name));
    // Equal editable parts would make both names collide, so the node keeps a trailing "_" as Python did.
    let collision = same_name(&node_new, &swap_name);
    let node_final = if collision { format!("{node_new}_") } else { node_new };
    validate_component(&node_final)?;
    validate_component(&swap_new)?;
    let pair = [name.as_str(), swap_name.as_str()];
    ensure_free(&parent_abs, get_node(&data, parent), &node_final, &pair)?;
    ensure_free(&parent_abs, get_node(&data, parent), &swap_new, &pair)?;

    let mut journal = Journal::new();
    journal.rename(&parent_abs.join(name), &parent_abs.join(&node_final))?;
    if let Err(e) = journal.rename(&parent_abs.join(&swap_name), &parent_abs.join(&swap_new)) {
        return Err(journal.abort("move", e));
    }
    if let Some(siblings) = get_node_mut(&mut data, parent) {
        let node_entry = siblings.remove(name.as_str());
        let swap_entry = siblings.remove(swap_name.as_str());
        if let Some(entry) = node_entry {
            siblings.insert(node_final.clone().into(), entry);
        }
        if let Some(entry) = swap_entry {
            siblings.insert(swap_new.into(), entry);
        }
    }
    persist(root, &data, journal, "move")?;
    let warnings = if collision {
        vec![format!("{name} became {node_final} because {swap_name} has the same name")]
    } else {
        Vec::new()
    };
    finish(root, &data, synced, Some(child_rel(parent, &node_final)), warnings)
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

pub fn delete_node(
    root: &Path,
    rel: &[String],
    expect_ghost: bool,
    confirm_non_empty: bool,
) -> R<Mutation> {
    delete_node_with(root, rel, expect_ghost, confirm_non_empty, send_to_recycle_bin)
}

pub(crate) fn delete_node_with(
    root: &Path,
    rel: &[String],
    expect_ghost: bool,
    confirm_non_empty: bool,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> R<Mutation> {
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    if target.ghost != expect_ghost {
        return Err(FmError::Stale { rel_parts: rel.to_vec() });
    }
    guard_delete(rel, &target)?;
    let (name, parent) = split(rel)?;
    if target.ghost {
        delete_key(&mut data, parent, name);
        yaml_store::save(root, &data)?;
        return finish(root, &data, synced, None, Vec::new());
    }
    let (files, dirs) = count_contents(&target.abs)?;
    if files + dirs > 0 && !confirm_non_empty {
        return Err(FmError::NeedsConfirmation { files, dirs });
    }

    let mut journal = Journal::new();
    let staged = stage_in_trash(root, &mut journal, &target.abs, name)?;
    delete_key(&mut data, parent, name);
    persist(root, &data, journal, "delete")?;

    let mut warnings = Vec::new();
    match recycle(&staged) {
        Ok(()) => remove_trash_if_empty(root, &mut warnings),
        Err(e) => warnings.push(format!(
            "{name} was moved to {} but not to the Recycle Bin: {e}",
            staged.display()
        )),
    }
    finish(root, &data, synced, None, warnings)
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

// Links are counted but never followed, so the preview cannot wander outside the folder.
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
                if !is_link(&meta) {
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
        Ok(meta) if !meta.is_dir() || is_link(&meta) => {
            return Err(FmError::InvalidPath {
                rel_parts: vec![TRASH_DIR.to_string()],
                reason: "not a plain folder".to_string(),
            });
        }
        Ok(_) => {}
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let free = (0..100)
        .map(|n| if n == 0 { format!("{stamp}-{name}") } else { format!("{stamp}-{n}-{name}") })
        .map(|file| trash.join(file))
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

fn remove_trash_if_empty(root: &Path, warnings: &mut Vec<String>) {
    let trash = root.join(TRASH_DIR);
    let empty = fs::read_dir(&trash).map(|mut entries| entries.next().is_none());
    match empty {
        Ok(true) => {
            if let Err(e) = fs::remove_dir(&trash) {
                warnings.push(format!("Could not remove {}: {e}", trash.display()));
            }
        }
        Ok(false) => {}
        Err(e) => warnings.push(format!("Could not inspect {}: {e}", trash.display())),
    }
}

// Removable and network drives have no Recycle Bin, where Windows would delete without asking.
#[cfg(windows)]
const RECYCLE_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $out = [IO.StreamWriter]::new([Console]::OpenStandardOutput(), [Text.UTF8Encoding]::new($false)); \
    try { \
        $path = $env:ANT_RECYCLE_PATH; \
        if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($path)).DriveType -ne 'Fixed') { throw 'This drive has no Recycle Bin' }; \
        Add-Type -AssemblyName Microsoft.VisualBasic; \
        [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory($path, \
            [Microsoft.VisualBasic.FileIO.UIOption]::OnlyErrorDialogs, \
            [Microsoft.VisualBasic.FileIO.RecycleOption]::SendToRecycleBin); \
        if (Test-Path -LiteralPath $path) { throw 'The folder is still there' }; \
        exit 0 \
    } catch { $out.WriteLine($_.Exception.Message); $out.Flush(); exit 1 }";

#[cfg(windows)]
fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    crate::fs_safe::run_powershell(RECYCLE_SCRIPT, &[("ANT_RECYCLE_PATH", path.as_os_str())], None)
        .map(|_| ())
}

#[cfg(not(windows))]
fn send_to_recycle_bin(_path: &Path) -> Result<(), String> {
    Err("the Recycle Bin is only available on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_actions::test_support::*;
    use crate::yaml_store::get_node;

    const NODES: &str = "100.WORK:\n  type: NODE-L1\n  01.A:\n    type: NODE-L2\n    01.Keep: {type: NODE-L3}\n  \
        02.B:\n    type: NODE-L2\n  03.C:\n    type: NODE-L2\n  04.Wkb:\n    type: WKB\n  \
        05.Ghost:\n    type: NODE-L2\n  98.Rev:\n    type: REV-L2\n";
    const NODE_DIRS: &[&str] =
        &["100.WORK/01.A/01.Keep", "100.WORK/02.B", "100.WORK/03.C", "100.WORK/04.Wkb", "100.WORK/98.Rev"];

    fn work(tag: &str) -> crate::yaml_store::test_support::TempDir {
        storage(tag, NODE_DIRS, NODES)
    }

    fn ok_recycle(path: &Path) -> Result<(), String> {
        assert!(path.parent().unwrap().ends_with(TRASH_DIR), "{}", path.display());
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    }

    #[test]
    fn move_down_swaps_ids_with_the_next_movable_sibling() {
        let dir = work("move-down");
        let m = move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down).unwrap();
        assert_eq!(m.select, Some(rel(&["100.WORK", "02.A"])));
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.B", "02.A", "03.C", "04.Wkb", "98.Rev"]);
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "02.A", "01.Keep"]), "NODE-L3");
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "01.B"]), "NODE-L2");
        assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "01.A"])).is_none());
        let up = move_node(dir.path(), &rel(&["100.WORK", "03.C"]), MoveDirection::Up).unwrap();
        assert_eq!(up.select, Some(rel(&["100.WORK", "02.C"])));
        assert!(dir.path().join("100.WORK/03.A").is_dir());
    }

    #[test]
    fn move_refuses_ends_ghosts_and_non_node_types() {
        let dir = work("move-refuse");
        let w = |p: &str, d| move_node(dir.path(), &rel(&["100.WORK", p]), d);
        assert!(matches!(w("01.A", MoveDirection::Up), Err(FmError::NotMovable)));
        assert!(matches!(w("03.C", MoveDirection::Down), Err(FmError::NotMovable)));
        assert!(matches!(w("04.Wkb", MoveDirection::Up), Err(FmError::NotMovable)));
        assert!(matches!(w("05.Ghost", MoveDirection::Up), Err(FmError::GhostNotAllowed { .. })));
        assert!(matches!(w("98.Rev", MoveDirection::Up), Err(FmError::Locked { .. })));
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
    }

    #[test]
    fn move_with_equal_names_uses_the_underscore_name() {
        let dir = storage(
            "move-clash",
            &["100.WORK/05.Ships", "100.WORK/06.ships"],
            "100.WORK:\n  05.Ships: {type: NODE-L2, x: {type: NODE-L3}}\n  06.ships: {type: NODE-L2}\n",
        );
        let m = move_node(dir.path(), &rel(&["100.WORK", "05.Ships"]), MoveDirection::Down).unwrap();
        assert_eq!(m.select, Some(rel(&["100.WORK", "06.Ships_"])));
        assert_eq!(m.warnings.len(), 1);
        assert_eq!(names(&dir.path().join("100.WORK")), ["05.ships", "06.Ships_"]);
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "06.Ships_", "x"]), "NODE-L3");
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "05.ships"]), "NODE-L2");
    }

    #[test]
    fn move_rolls_back_when_yaml_save_fails() {
        let dir = work("move-yaml");
        break_yaml_save(dir.path());
        let err = move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down).unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
    }

    // A handle without FILE_SHARE_DELETE on the sibling makes the second rename fail mid-move.
    #[cfg(windows)]
    #[test]
    fn move_undoes_the_first_rename_when_the_second_fails() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const FILE_SHARE_READ_WRITE: u32 = 0x1 | 0x2;
        let dir = work("move-second");
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(dir.path().join("100.WORK/02.B"))
            .unwrap();
        let err = move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down).unwrap_err();
        drop(held);
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "01.A"]), "NODE-L2");
    }

    #[test]
    fn delete_ghost_only_touches_yaml_and_checks_expectation() {
        let dir = work("delete-ghost");
        let ghost = rel(&["100.WORK", "05.Ghost"]);
        let stale = delete_node_with(dir.path(), &ghost, false, false, ok_recycle);
        assert!(matches!(stale, Err(FmError::Stale { .. })));
        let real = delete_node_with(dir.path(), &rel(&["100.WORK", "02.B"]), true, false, ok_recycle);
        assert!(matches!(real, Err(FmError::Stale { .. })));
        assert_eq!(inspect_delete(dir.path(), &ghost).unwrap(), DeletePreview { ghost: true, files: 0, dirs: 0 });
        let m = delete_node_with(dir.path(), &ghost, true, false, |_| panic!("no disk delete")).unwrap();
        assert!(m.select.is_none());
        assert!(get_node(&yaml(dir.path()), &ghost).is_none());
        assert!(dir.path().join("100.WORK/02.B").is_dir());
    }

    #[test]
    fn delete_empty_folder_goes_through_the_trash() {
        let dir = work("delete-empty");
        let p = rel(&["100.WORK", "02.B"]);
        assert_eq!(inspect_delete(dir.path(), &p).unwrap(), DeletePreview { ghost: false, files: 0, dirs: 0 });
        let m = delete_node_with(dir.path(), &p, false, false, ok_recycle).unwrap();
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
        let refused = delete_node_with(dir.path(), &p, false, false, ok_recycle);
        assert!(matches!(refused, Err(FmError::NeedsConfirmation { files: 1, dirs: 1 })));
        assert!(dir.path().join("100.WORK/01.A/01.Keep/notes.txt").is_file());

        let m = delete_node_with(dir.path(), &p, false, true, |_| Err("bin unavailable".to_string())).unwrap();
        assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
        let staged = names(&dir.path().join(TRASH_DIR));
        assert_eq!(staged.len(), 1);
        assert!(staged[0].ends_with("-01.A"), "{staged:?}");
        let kept = dir.path().join(TRASH_DIR).join(&staged[0]).join("01.Keep/notes.txt");
        assert_eq!(fs::read_to_string(kept).unwrap(), "keep me");
        assert!(get_node(&yaml(dir.path()), &p).is_none());
    }

    #[test]
    fn delete_puts_the_folder_back_when_yaml_save_fails() {
        let dir = work("delete-rollback");
        break_yaml_save(dir.path());
        let p = rel(&["100.WORK", "02.B"]);
        let err = delete_node_with(dir.path(), &p, false, false, |_| panic!("not staged")).unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert!(dir.path().join("100.WORK/02.B").is_dir());
        assert!(!dir.path().join(TRASH_DIR).exists());
    }

    #[test]
    fn delete_refuses_reserved_and_locked_folders_but_not_locked_ghosts() {
        let dir = storage("delete-refuse", &["100.WORK/98.Rev", "998.Review"], "100.WORK:\n  97.Gone: {type: TPL}\n");
        for reserved in ["999.Settings", "998.Review"] {
            let err = delete_node_with(dir.path(), &rel(&[reserved]), false, true, ok_recycle);
            assert!(matches!(err, Err(FmError::Reserved { .. })), "{reserved}");
        }
        let locked = delete_node_with(dir.path(), &rel(&["100.WORK", "98.Rev"]), false, true, ok_recycle);
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        delete_node_with(dir.path(), &rel(&["100.WORK", "97.Gone"]), true, false, ok_recycle).unwrap();
        assert!(dir.path().join("100.WORK/98.Rev").is_dir() && dir.path().join("999.Settings").is_dir());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "starts PowerShell"]
    fn recycle_reports_a_missing_folder_as_an_error() {
        let dir = work("recycle-missing");
        let err = send_to_recycle_bin(&dir.path().join("does-not-exist")).unwrap_err();
        assert!(err.contains("PowerShell failed") && err.contains("does-not-exist"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "sends a temp folder to the real Recycle Bin through PowerShell"]
    fn delete_sends_the_staged_folder_to_the_recycle_bin() {
        let dir = work("delete-real");
        let m = delete_node(dir.path(), &rel(&["100.WORK", "02.B"]), false, false).unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        assert!(!dir.path().join(TRASH_DIR).exists());
    }
}
