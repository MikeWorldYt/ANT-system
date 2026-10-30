// Decides which folder receives files dropped on a tree node, following the intake rules for loose files.
use serde_yaml::Mapping;

use crate::ant_core::tree_types::{flags_in, type_base};
use crate::config::DATA_FLAG;
use crate::error::{FmError, R};
use crate::intake::{node_type, review_child};
use crate::mutations::child_rel;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::is_locked_type;

pub(crate) fn destination(rel: &[String], node: &Mapping) -> R<Vec<String>> {
    let refuse = |reason: &str| FmError::InvalidPath { rel_parts: rel.to_vec(), reason: reason.to_string() };
    let Some(top) = rel.first() else {
        return Err(refuse("files cannot be dropped on the storage root"));
    };
    // Everything under a reserved root belongs to the app, not only the root folder itself.
    if is_reserved(std::slice::from_ref(top)) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    let raw = node_type(node);
    if type_base(raw) == "REV" {
        return Ok(rel.to_vec());
    }
    if is_locked_type(raw) {
        return Err(FmError::Locked { rel_parts: rel.to_vec(), node_type: raw.to_string() });
    }
    match type_base(raw) {
        "WKB" | "PORTF" => Ok(rel.to_vec()),
        "NODE" if flags_in(node).iter().any(|f| f == DATA_FLAG) => Ok(rel.to_vec()),
        "NODE" => review_child(node)
            .map(|review| child_rel(rel, review))
            .ok_or_else(|| refuse("the folder has no 98.* review folder to receive the files")),
        "" => Err(refuse("the folder has no type in the tree yet; refresh and retry")),
        other => Err(refuse(&format!("{other} folders do not take dropped files"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rel;

    fn node(yaml: &str) -> Mapping {
        serde_yaml::from_str(yaml).unwrap()
    }

    fn dest(parts: &[&str], yaml: &str) -> R<Vec<String>> {
        destination(&rel(parts), &node(yaml))
    }

    const WITH_REVIEW: &str = "type: NODE-L2\n98.Toki: {type: REV-L2}\n01.Sub: {type: NODE-L3}\n";

    #[test]
    fn a_data_node_and_a_workboard_take_the_files_themselves() {
        let data = dest(&["100.A", "04.Data"], "type: NODE-L2\nflags: data\n98.Data: {type: REV-L2}\n").unwrap();
        assert_eq!(data, rel(&["100.A", "04.Data"]));
        assert_eq!(dest(&["100.A", "PJ.001_Web"], "type: WKB-L2\n").unwrap(), rel(&["100.A", "PJ.001_Web"]));
    }

    #[test]
    fn a_node_without_data_sends_the_files_to_its_review_child() {
        assert_eq!(dest(&["100.A", "03.Toki"], WITH_REVIEW).unwrap(), rel(&["100.A", "03.Toki", "98.Toki"]));
        let main = "type: NODE-L1\nflags: main\n98.A: {type: REV-L1}\n";
        assert_eq!(dest(&["100.A"], main).unwrap(), rel(&["100.A", "98.A"]));
        let other_flag = "type: NODE-L2\nflags: user\n98.B: {type: REV-L2}\n";
        assert_eq!(dest(&["100.A", "02.B"], other_flag).unwrap(), rel(&["100.A", "02.B", "98.B"]));
    }

    #[test]
    fn a_node_without_a_review_child_is_refused() {
        let err = dest(&["100.A", "05.Bare"], "type: NODE-L2\n98.Fake: {type: NODE-L3}\n").unwrap_err();
        assert!(matches!(&err, FmError::InvalidPath { reason, .. } if reason.contains("98.*")), "{err:?}");
    }

    #[test]
    fn locked_types_other_than_review_are_refused() {
        for t in ["RES", "TPL", "CFG"] {
            let err = dest(&["100.A", "98.A"], &format!("type: {t}\n")).unwrap_err();
            assert!(matches!(&err, FmError::Locked { node_type, .. } if node_type == t), "{t}: {err:?}");
        }
    }

    #[test]
    fn a_review_folder_takes_the_files_itself_unless_it_is_reserved() {
        assert_eq!(dest(&["100.A", "03.Toki", "98.Toki"], "type: REV-L3
").unwrap(), rel(&["100.A", "03.Toki", "98.Toki"]));
        for parts in [&["998.Review", "98.Old"][..], &["999.Settings", "98.Cfg"]] {
            let err = dest(parts, "type: REV-L2
").unwrap_err();
            assert!(matches!(err, FmError::Reserved { .. }), "{parts:?}: {err:?}");
        }
    }

    #[test]
    fn reserved_folders_and_their_contents_are_refused() {
        for parts in [&["999.Settings"][..], &["998.review"], &["999.Settings", "00.Core"]] {
            let err = dest(parts, "type: NODE-L1\nflags: data\n").unwrap_err();
            assert!(matches!(err, FmError::Reserved { .. }), "{parts:?}: {err:?}");
        }
    }

    #[test]
    fn the_root_and_untyped_folders_are_refused() {
        assert!(matches!(dest(&[], "type: NODE-L1\n"), Err(FmError::InvalidPath { .. })));
        assert!(matches!(dest(&["100.A", "X"], "{}"), Err(FmError::InvalidPath { .. })));
    }
}
