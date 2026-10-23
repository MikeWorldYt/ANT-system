// Reorder nodes by swapping their numeric ids, and delete folders (via .ant-trash and the Recycle Bin) or ghost records.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::TRASH_DIR;
use crate::edit_actions::{
    child_rel, ensure_free, finish, guard_frozen, load_synced, locate, persist, split, Mutation,
    Target,
};
use crate::error::{FmError, R};
use crate::fs_safe::{
    io_err, join_rel, redirects, same_name, validate_component, Journal, StorageLock,
    MAX_NAME_UNITS,
};
use crate::names::{editable_part, is_movable_node, list_subdirs, numeric_id};
use crate::tree_node::is_reserved;
use crate::tree_store::{self, delete_key, get_node, get_node_mut, get_type, is_locked_type};

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

// A folder already staged in .ant-trash and dropped from the tree; recycling it needs no storage lock.
pub struct PendingRecycle {
    root: PathBuf,
    staged: PathBuf,
    name: String,
}

// Staged folders still on their way to the Recycle Bin; anything else in .ant-trash was restored or kept.
static IN_FLIGHT: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

struct InFlight(PathBuf);

impl Drop for InFlight {
    fn drop(&mut self) {
        IN_FLIGHT.lock().unwrap_or_else(|p| p.into_inner()).retain(|p| *p != self.0);
    }
}

// The Recycle Bin restores a staged folder into .ant-trash, where it is hidden, so refresh moves it to the storage root under its own name.
pub fn recover_trashed(root: &Path) -> R<Vec<String>> {
    let trash = root.join(TRASH_DIR);
    let entries = match fs::read_dir(&trash) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err("list folder", &trash, &e)),
    };
    let mut recovered = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| io_err("list folder", &trash, &e))?.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e))?;
        let in_flight = IN_FLIGHT.lock().unwrap_or_else(|p| p.into_inner()).contains(&path);
        if in_flight || !meta.is_dir() || redirects(&path, &meta)? {
            continue;
        }
        let Some(original) = path.file_name().and_then(|n| n.to_str()).and_then(original_name) else { continue };
        let taken = list_subdirs(root)?;
        let free = (0..100)
            .map(|n| if n == 0 { original.to_string() } else { format!("{original}_{n}") })
            .find(|c| !taken.iter().any(|t| same_name(t, c)) && validate_component(c).is_ok());
        let Some(target) = free else { continue };
        fs::rename(&path, root.join(&target)).map_err(|e| io_err("restore folder", &path, &e))?;
        recovered.push(target);
    }
    remove_trash_if_empty(root, &mut Vec::new());
    Ok(recovered)
}

// Inverse of `trash_name`: drops "<stamp>-" and the optional "<n>-" counter.
fn original_name(staged: &str) -> Option<&str> {
    let (stamp, rest) = staged.split_once('-')?;
    if stamp.is_empty() || !stamp.bytes().all(|b| b.is_ascii_digit() || b.is_ascii_lowercase()) {
        return None;
    }
    let after_counter = rest.split_once('-').filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    Some(after_counter.map_or(rest, |(_, name)| name)).filter(|n| !n.is_empty())
}

pub fn move_node(root: &Path, rel: &[String], dir: MoveDirection) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
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
    Ok(finish(root, &data, synced, Some(child_rel(parent, &node_final)), warnings))
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

// Returns warnings for the Mutation; on any doubt the staged copy stays in .ant-trash.
pub fn recycle_staged(pending: PendingRecycle) -> Vec<String> {
    recycle_staged_with(pending, send_to_recycle_bin)
}

pub(crate) fn recycle_staged_with(
    pending: PendingRecycle,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> Vec<String> {
    let PendingRecycle { root, staged, name } = pending;
    let _in_flight = InFlight(staged.clone());
    let kept =|why: &str| {
        vec![format!("{name} left the tree but was not sent to the Recycle Bin; it is kept in {}: {why}", staged.display())]
    };
    if let Err(why) = check_staged(&root, &staged) {
        return kept(&why);
    }
    match recycle(&staged) {
        Err(why) => kept(&why),
        Ok(()) if fs::symlink_metadata(&staged).is_ok() => kept("the folder is still there"),
        Ok(()) => {
            let mut warnings = Vec::new();
            remove_trash_if_empty(&root, &mut warnings);
            warnings
        }
    }
}

// The folder sat unlocked since staging, so it must still be a plain folder directly inside .ant-trash.
fn check_staged(root: &Path, staged: &Path) -> Result<(), String> {
    if staged.parent() != Some(root.join(TRASH_DIR).as_path()) {
        return Err("it is not inside the staging folder".to_string());
    }
    let meta = fs::symlink_metadata(staged).map_err(|e| e.to_string())?;
    match redirects(staged, &meta) {
        Ok(false) if meta.is_dir() => Ok(()),
        Ok(_) => Err("it is no longer a plain folder".to_string()),
        Err(e) => Err(e.to_string()),
    }
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
fn trash_name(stamp: &str, n: usize, name: &str) -> String {
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

// SHFileOperationW reports failed or cancelled recycles; flags 0x4454 (ALLOWUNDO|NOCONFIRMATION|WANTNUKEWARNING|NOERRORUI|SILENT) leave only the permanent-delete warning.
#[cfg(windows)]
const RECYCLE_TYPE: &str = r#"
using System;
using System.Runtime.InteropServices;
public static class AntRecycleBin {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SHFILEOPSTRUCTW {
        public IntPtr hwnd;
        public uint wFunc;
        public string pFrom;
        public string pTo;
        public ushort fFlags;
        [MarshalAs(UnmanagedType.Bool)] public bool fAnyOperationsAborted;
        public IntPtr hNameMappings;
        public string lpszProgressTitle;
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern int SHFileOperationW(ref SHFILEOPSTRUCTW op);
    public static string Send(string path) {
        if (IntPtr.Size != 8) { return "only 64-bit PowerShell is supported"; }
        SHFILEOPSTRUCTW op = new SHFILEOPSTRUCTW();
        op.wFunc = 3;
        op.pFrom = path + "\0";
        op.fFlags = 0x4454;
        int code = SHFileOperationW(ref op);
        if (code != 0) { return "SHFileOperation failed with code 0x" + code.ToString("X"); }
        if (op.fAnyOperationsAborted) { return "the operation was cancelled"; }
        return "";
    }
}
"#;

// Removable and network drives have no Recycle Bin, where Windows would delete without asking.
#[cfg(windows)]
const RECYCLE_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $out = [IO.StreamWriter]::new([Console]::OpenStandardOutput(), [Text.UTF8Encoding]::new($false)); \
    try { \
        $path = $env:ANT_RECYCLE_PATH; \
        if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($path)).DriveType -ne 'Fixed') { throw 'This drive has no Recycle Bin' }; \
        if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw ('The folder is missing: ' + $path) }; \
        Add-Type -TypeDefinition $env:ANT_RECYCLE_TYPE; \
        $failure = [AntRecycleBin]::Send($path); \
        if ($failure) { throw $failure }; \
        if (Test-Path -LiteralPath $path) { throw 'The folder is still there' }; \
        exit 0 \
    } catch { $out.WriteLine($_.Exception.Message); $out.Flush(); exit 1 }";

#[cfg(windows)]
const RECYCLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

#[cfg(windows)]
fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    let env = [("ANT_RECYCLE_PATH", path.as_os_str()), ("ANT_RECYCLE_TYPE", std::ffi::OsStr::new(RECYCLE_TYPE))];
    crate::fs_safe::run_powershell(RECYCLE_SCRIPT, &env, None, RECYCLE_TIMEOUT).map(|_| ())
}

#[cfg(not(windows))]
fn send_to_recycle_bin(_path: &Path) -> Result<(), String> {
    Err("the Recycle Bin is only available on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_actions::test_support::*;
    use crate::tree_store::get_node;

    const NODES: &str = "100.WORK:\n  type: NODE-L1\n  01.A:\n    type: NODE-L2\n    01.Keep: {type: NODE-L3}\n  \
        02.B:\n    type: NODE-L2\n  03.C:\n    type: NODE-L2\n  04.Wkb:\n    type: WKB\n  \
        05.Ghost:\n    type: NODE-L2\n  98.Rev:\n    type: REV-L2\n";
    const NODE_DIRS: &[&str] =
        &["100.WORK/01.A/01.Keep", "100.WORK/02.B", "100.WORK/03.C", "100.WORK/04.Wkb", "100.WORK/98.Rev"];

    fn work(tag: &str) -> crate::tree_store::test_support::TempDir {
        storage(tag, NODE_DIRS, NODES)
    }

    fn ok_recycle(path: &Path) -> Result<(), String> {
        assert!(path.parent().unwrap().ends_with(TRASH_DIR), "{}", path.display());
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    }

    // What the command layer does: delete under the lock, then recycle after releasing it.
    fn delete_with(
        root: &Path,
        rel: &[String],
        expect_ghost: bool,
        confirm: bool,
        recycle: impl FnOnce(&Path) -> Result<(), String>,
    ) -> R<Mutation> {
        let (mut m, pending) = delete_node(root, rel, expect_ghost, confirm)?;
        if let Some(pending) = pending {
            m.warnings.extend(recycle_staged_with(pending, recycle));
        }
        Ok(m)
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

    #[cfg(windows)]
    #[test]
    fn move_rolls_back_when_yaml_save_fails() {
        let dir = work("move-yaml");
        let _held = break_tree_save(dir.path());
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
    fn recycle_runs_without_the_storage_lock_after_the_yaml_commit() {
        let dir = work("recycle-unlocked");
        let p = rel(&["100.WORK", "02.B"]);
        let m = delete_with(dir.path(), &p, false, false, |staged| {
            assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "02.B"])).is_none());
            drop(StorageLock::acquire(dir.path()).map_err(|e| e.to_string())?);
            ok_recycle(staged)
        })
        .unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    }

    #[test]
    fn recycle_that_leaves_the_folder_in_place_keeps_it_with_a_warning() {
        let dir = work("recycle-noop");
        let m = delete_with(dir.path(), &rel(&["100.WORK", "02.B"]), false, false, |_| Ok(())).unwrap();
        assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
        assert!(m.warnings[0].contains("still there"), "{:?}", m.warnings);
        assert_eq!(names(&dir.path().join(TRASH_DIR)).len(), 1);
    }

    #[test]
    fn recycle_refuses_a_path_outside_the_staging_folder() {
        let dir = work("recycle-outside");
        let staged = dir.path().join("100.WORK/02.B");
        let pending = PendingRecycle { root: dir.path().to_path_buf(), staged, name: "02.B".to_string() };
        let warnings = recycle_staged_with(pending, |_| panic!("must not recycle"));
        assert_eq!(warnings.len(), 1);
        assert!(dir.path().join("100.WORK/02.B").is_dir());
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
        crate::tree_store::save(dir.path(), &data).unwrap();
        let p = rel(&["100.work", "02.B"]);
        assert!(matches!(delete_with(dir.path(), &p, false, true, ok_recycle), Err(FmError::Stale { .. })));
        delete_with(dir.path(), &p, true, false, |_| panic!("no disk delete")).unwrap();
        assert!(dir.path().join("100.WORK/02.B").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "02.B"]), "NODE-L2");
    }

    #[test]
    fn original_name_inverts_trash_name() {
        for (name, n) in [("01.A", 0), ("01.A", 3), ("02.Plan-b", 0)] {
            assert_eq!(original_name(&trash_name("lz4k9", n, name)), Some(name));
        }
        assert_eq!(original_name("nodash"), None);
        assert_eq!(original_name("Bad!-01.A"), None);
    }

    #[test]
    fn restored_folders_come_back_to_the_root_but_in_flight_ones_stay() {
        let dir = work("recover");
        let p = rel(&["100.WORK", "02.B"]);
        fs::write(dir.path().join("100.WORK/02.B/n.txt"), "x").unwrap();
        let (_, pending) = delete_node(dir.path(), &p, false, true).unwrap();
        assert!(recover_trashed(dir.path()).unwrap().is_empty());
        assert_eq!(names(&dir.path().join(TRASH_DIR)).len(), 1);
        // Simulates the Recycle Bin restoring the folder under its staged name.
        let staged = names(&dir.path().join(TRASH_DIR)).remove(0);
        drop(recycle_staged_with(pending.unwrap(), |_| Ok(())));
        fs::create_dir(dir.path().join("02.B")).unwrap();
        assert_eq!(recover_trashed(dir.path()).unwrap(), ["02.B_1"]);
        assert!(staged.ends_with("-02.B"));
        assert_eq!(fs::read_to_string(dir.path().join("02.B_1/n.txt")).unwrap(), "x");
        assert!(!dir.path().join(TRASH_DIR).exists());
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
        let outside = crate::tree_store::test_support::TempDir::new("count-outside");
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
        let (m, pending) = delete_node(dir.path(), &rel(&["100.WORK", "02.B"]), false, false).unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        let warnings = recycle_staged(pending.unwrap());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(!dir.path().join(TRASH_DIR).exists());
    }
}
