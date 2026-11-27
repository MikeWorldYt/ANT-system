// Which support folders a node may hold and what each is called, from its type, level and the storage depth limit.
use crate::domain::config::{MEDIA_NODES, SUPPORT_NODES};
use crate::domain::names::{editable_part, has_node_id, is_main_category};
use crate::domain::tree_types::type_base;

// A category's resources start as one shared folder; deeper levels name theirs after the node.
const CATEGORY_RESOURCES: &str = "RES.Resources";

pub fn allowed_kinds(name: &str, node_type: &str, level: usize, limit: usize) -> Vec<&'static str> {
    let numbered = if level == 1 { is_main_category(name) } else { has_node_id(name) };
    if type_base(node_type) != "NODE" || !numbered || level > limit {
        return Vec::new();
    }
    let mut kinds = vec!["REV"];
    // The tree stops at the depth limit, so a deepest node keeps only its review folder, on disk.
    // Only a main category gets screenshots and recordings by default; deeper nodes get them when the user adds them.
    if level < limit {
        kinds.extend(["RES", "TPL", "SS", "REC"]);
    }
    kinds
}

pub fn support_folder_name(kind: &str, node_name: &str, level: usize) -> Option<String> {
    if kind == "RES" && level == 1 {
        return Some(CATEGORY_RESOURCES.to_string());
    }
    let words = SUPPORT_NODES.iter().map(|(word, _, kind)| (*word, *kind)).chain(MEDIA_NODES.iter().map(|(word, kind, _)| (*word, *kind)));
    let (word, _) = words.into_iter().find(|(_, own)| *own == kind)?;
    Some(format!("{word}{}", editable_part(node_name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::names::support_kind;

    #[test]
    fn kinds_follow_the_level_and_the_node_type() {
        assert_eq!(allowed_kinds("100.WORK", "NODE-L1", 1, 3), ["REV", "RES", "TPL", "SS", "REC"]);
        assert_eq!(allowed_kinds("01.Math", "NODE-L2", 2, 3), ["REV", "RES", "TPL", "SS", "REC"]);
        assert_eq!(allowed_kinds("01.Algebra", "NODE-L3", 3, 3), ["REV"]);
        assert_eq!(allowed_kinds("01.Algebra", "NODE-L3", 3, 4), ["REV", "RES", "TPL", "SS", "REC"]);
    }

    #[test]
    fn worknodes_support_folders_and_plain_folders_hold_none() {
        for (name, kind, level) in [("PJ.001_Logo", "WKB", 2), ("00.Outcomes", "OUT", 2), ("REV.Math", "REV-L2", 3), ("RES.Math", "RES", 3), ("CFG", "CFG", 1), ("01.Math", "", 2)] {
            assert!(allowed_kinds(name, kind, level, 3).is_empty(), "{name}");
        }
        assert!(allowed_kinds("Loose", "NODE-L1", 1, 3).is_empty());
        assert!(allowed_kinds("01.Math", "NODE-L4", 4, 3).is_empty());
        assert!(allowed_kinds("ANT.SCREENSHOTS", "SS", 1, 3).is_empty());
    }

    #[test]
    fn names_read_back_as_their_own_kind() {
        for (kind, level, want) in [("REV", 2, "REV.Math"), ("RES", 2, "RES.Math"), ("TPL", 2, "TPL.Math"), ("RES", 1, "RES.Resources")] {
            let name = support_folder_name(kind, if level == 1 { "100.WORK" } else { "01.Math" }, level).unwrap();
            assert_eq!(name, want);
            assert_eq!(support_kind(&name), Some(kind));
        }
        assert_eq!(support_folder_name("SS", "100.WORK", 1).as_deref(), Some("SS.WORK"));
        assert_eq!(support_folder_name("REC", "100.WORK", 1).as_deref(), Some("REC.WORK"));
        assert_eq!(support_folder_name("CFG", "100.WORK", 1), None);
    }
}
