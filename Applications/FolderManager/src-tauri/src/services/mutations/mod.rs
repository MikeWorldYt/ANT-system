// Rename, retype and add nodes, keeping disk folders, linked 90./98. folders and the tree in step.
mod add;
mod board;
mod checks;
mod copy_answer;
mod delete;
mod favorite;
mod hidden;
mod important;
mod file_drop;
mod file_ops;
mod link_hidden;
mod outputs;
mod move_folder;
mod mutation;
mod user_flags;
mod recycle;
mod rename;
mod reorder;
mod result;
mod resolve;
mod retype;
mod review_filter;
mod save;
mod target;

pub use add::{add_next, add_node};
pub use board::{set_board, BoardCard};
pub use copy_answer::{answer_copy, CopyAnswer};
pub use delete::{delete_node, inspect_delete, DeletePreview};
pub use favorite::set_favorite;
pub use important::set_important;
pub use file_ops::{delete_file, file_path, rename_file};
pub use link_hidden::set_link_hidden;
pub use file_drop::{drop_files, DropOutcome};
pub(crate) use crate::infra::naming::free_name;
pub(crate) use crate::tree::tree_store::load_synced;
pub use move_folder::{move_folder, move_to_review};
pub use user_flags::set_user_flag;
pub(crate) use crate::platform::recycle::send_to_recycle_bin;
pub use rename::rename_node;
pub use reorder::{move_node, MoveDirection};
pub use result::LevelPatch;
pub(crate) use result::finish;
pub use resolve::{preview_names, resolve_invalid, Pending, Resolution};
pub use hidden::{hidden_choices, set_hidden, set_hidden_children, HiddenChoice};
pub use outputs::{deliver, restore, set_output};
pub use retype::{set_details, set_node_type};
pub use review_filter::set_review_filter;
pub use mutation::Mutation;
pub(crate) use mutation::subtrees;
pub(crate) use checks::{ensure_free, guard_frozen};
pub(crate) use save::{load_synced_at, persist, register_ids};
pub(crate) use target::{child_rel, locate, parent_of, split, Target};
use target::exactly_on_disk;

#[cfg(test)]
mod tests {
    use serde_yaml::{Mapping, Value};

    use super::*;
    use crate::config::{EditableType, NewKind};
    use crate::error::{FmError, R};
    use crate::fs_safe::StorageLock;
    use crate::test_support::*;
    use crate::tree::tree_store::get_node;

    #[test]
    fn meta_key_folders_are_refused_and_the_parent_type_survives() {
        let dir = academic("meta-key");
        dir.mkdir("100.ACADEMIC/type");
        dir.mkdir("100.ACADEMIC/01.Math/status");
        let refused = |r: R<Mutation>| matches!(r, Err(FmError::InvalidName { .. }));
        assert!(refused(rename_node(dir.path(), &rel(&["100.ACADEMIC", "type"]), "X")));
        assert!(refused(set_node_type(dir.path(), &rel(&["100.ACADEMIC", "type"]), EditableType::Wkb, None)));
        assert!(refused(add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "status"]), NewKind::Node)));
        assert!(dir.path().join("100.ACADEMIC/type").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC"]), "NODE-L1");
        let math = get_node(&yaml(dir.path()), &rel(&["100.ACADEMIC", "01.Math"])).unwrap().clone();
        assert_eq!(math.get("status").and_then(Value::as_str), Some("done"));
    }

    #[test]
    fn locate_needs_every_component_with_its_exact_case() {
        let dir = academic("locate-case");
        let data: Mapping = serde_yaml::from_str("100.academic:\n  02.Bio: {type: NODE-L2}\n").unwrap();
        let target = locate(dir.path(), &data, &rel(&["100.academic", "02.Bio"])).unwrap();
        assert!(target.ghost);
        assert!(!locate(dir.path(), &data, &rel(&["100.ACADEMIC", "02.Bio"])).unwrap().ghost);
    }

    #[test]
    fn mutations_refuse_while_another_process_holds_the_lock() {
        let dir = academic("locked-storage");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let err = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap_err();
        assert!(matches!(err, FmError::Io { ref op, .. } if op == "lock storage"), "{err:?}");
        assert!(dir.path().join("100.ACADEMIC/02.Bio").is_dir());
        drop(held);
        rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap();
    }
}
