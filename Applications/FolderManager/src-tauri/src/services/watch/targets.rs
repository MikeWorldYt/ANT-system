// Maps changed paths to the tree nodes whose subtree sync picks the change up.
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::META_KEYS;

// A folder not yet in the tree maps to its nearest known ancestor, whose subtree sync adds it.
pub(super) fn nearest_node(data: &Mapping, rel: &[String]) -> Vec<String> {
    let mut node = data;
    let mut found = Vec::new();
    for part in rel {
        if META_KEYS.contains(&part.as_str()) {
            break;
        }
        let Some(Value::Mapping(child)) = node.get(part.as_str()) else { break };
        node = child;
        found.push(part.clone());
    }
    found
}

// A node inside another target's subtree is already covered by that target's sync.
pub(super) fn targets(data: &Mapping, rels: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut nodes: Vec<Vec<String>> = rels.iter().map(|rel| nearest_node(data, rel)).collect();
    nodes.sort();
    nodes.dedup();
    let mut kept: Vec<Vec<String>> = Vec::new();
    for node in nodes {
        if !kept.iter().any(|k| node.starts_with(k)) {
            kept.push(node);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rel;

    fn tree() -> Mapping {
        serde_yaml::from_str("100.WORK:\n  type: NODE-L1\n  01.A:\n    type: NODE-L2\n  02.B: {type: NODE-L2}\n").unwrap()
    }

    #[test]
    fn a_path_maps_to_itself_or_its_nearest_node() {
        let data = tree();
        assert_eq!(nearest_node(&data, &rel(&["100.WORK", "01.A"])), rel(&["100.WORK", "01.A"]));
        assert_eq!(nearest_node(&data, &rel(&["100.WORK", "06.New", "deep"])), rel(&["100.WORK"]));
        assert_eq!(nearest_node(&data, &rel(&["100.WORK", "01.A", "file.txt"])), rel(&["100.WORK", "01.A"]));
        assert_eq!(nearest_node(&data, &rel(&["100.WORK", "type"])), rel(&["100.WORK"]));
        assert_eq!(nearest_node(&data, &rel(&["200.NEW"])), Vec::<String>::new());
    }

    #[test]
    fn targets_inside_another_target_are_dropped() {
        let data = tree();
        let rels = [rel(&["100.WORK", "01.A", "x"]), rel(&["100.WORK", "06.New"]), rel(&["100.WORK", "02.B"])];
        assert_eq!(targets(&data, &rels), [rel(&["100.WORK"])]);
        let rels = [rel(&["100.WORK", "01.A", "x"]), rel(&["100.WORK", "02.B", "y"]), rel(&["100.WORK", "01.A"])];
        assert_eq!(targets(&data, &rels), [rel(&["100.WORK", "01.A"]), rel(&["100.WORK", "02.B"])]);
        assert_eq!(targets(&data, &[rel(&["200.NEW"]), rel(&["100.WORK", "01.A"])]), [Vec::<String>::new()]);
    }
}
