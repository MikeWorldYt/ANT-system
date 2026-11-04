use std::path::PathBuf;

use super::{drop_effect, folder_effect, paste_plan, ClipboardFiles, Paste, CUT_REFUSED, DROPEFFECT_COPY, DROPEFFECT_MOVE};
use crate::error::FmError;
use crate::test_support::rel;

fn files(effect: Option<u32>) -> ClipboardFiles {
    ClipboardFiles { paths: vec![PathBuf::from(r"C:\a.txt"), PathBuf::from(r"C:\b.txt")], effect }
}

#[test]
fn cut_files_are_moved() {
    let plan = paste_plan(files(Some(DROPEFFECT_MOVE))).unwrap();
    assert_eq!(plan, Paste { paths: files(None).paths, copy: false });
}

#[test]
fn any_other_effect_copies() {
    for effect in [None, Some(0), Some(1), Some(4), Some(5), Some(u32::MAX)] {
        assert!(paste_plan(files(effect)).unwrap().copy, "effect {effect:?}");
    }
}

#[test]
fn an_empty_clipboard_is_refused() {
    let err = paste_plan(ClipboardFiles { paths: Vec::new(), effect: Some(DROPEFFECT_MOVE) }).unwrap_err();
    assert!(matches!(err, FmError::ClipboardEmpty));
    assert_eq!(err.to_string(), "The clipboard has no files to paste");
}

#[test]
fn cut_writes_move_and_copy_writes_copy() {
    assert_eq!((drop_effect(true), drop_effect(false)), (2, 1));
    assert_eq!((DROPEFFECT_MOVE, DROPEFFECT_COPY), (2, 1));
}

#[test]
fn only_data_folders_and_workboards_can_be_cut() {
    let bio = rel(&["100.A", "02.Bio"]);
    let data = vec!["data".to_string()];
    assert_eq!(folder_effect(&bio, "NODE-L2", &data, false, true).unwrap(), DROPEFFECT_MOVE);
    assert_eq!(folder_effect(&bio, "WKB-L2", &[], false, true).unwrap(), DROPEFFECT_MOVE);
    for (node_type, flags) in [("NODE-L2", vec![]), ("NODE-L1", vec!["main".to_string()]), ("REV-L2", vec![]), ("", vec![])] {
        let err = folder_effect(&bio, node_type, &flags, false, true).unwrap_err();
        assert!(matches!(&err, FmError::InvalidPath { reason, .. } if reason == CUT_REFUSED), "{node_type}: {err:?}");
        assert_eq!(folder_effect(&bio, node_type, &flags, false, false).unwrap(), DROPEFFECT_COPY);
    }
}

#[test]
fn reserved_and_missing_folders_are_neither_copied_nor_cut() {
    for cut in [false, true] {
        let reserved = folder_effect(&rel(&["998.review"]), "NODE-L1", &["data".to_string()], false, cut).unwrap_err();
        assert!(matches!(reserved, FmError::Reserved { .. }), "{reserved:?}");
        let ghost = folder_effect(&rel(&["100.A", "PJ.Web"]), "WKB-L2", &[], true, cut).unwrap_err();
        assert!(matches!(ghost, FmError::GhostNotAllowed { .. }), "{ghost:?}");
    }
}
