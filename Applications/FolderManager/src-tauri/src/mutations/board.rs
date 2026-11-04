// Task board placement lives in each workboard's info.json, so a card keeps its column and order when the folder moves.
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml::Value;

use super::{finish, guard_frozen, load_synced_at, locate, Mutation};
use crate::ant_core::tree_types::type_base;
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::storage::info::{field_of, read_info, write_info};
use crate::tree::tree_store::{self, get_node};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardColumn {
    Backlog,
    Todo,
    Progress,
    Done,
}

impl BoardColumn {
    fn as_str(self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::Progress => "progress",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardCard {
    pub rel_parts: Vec<String>,
    pub column: BoardColumn,
    pub order: u32,
}

// The status a card had when placed is stored with it, so a status changed elsewhere is noticed and moves the card.
pub fn set_board(root: &Path, cards: &[BoardCard]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    if cards.is_empty() {
        return Err(FmError::Internal { message: "no cards to place on the board".to_string() });
    }
    let (data, synced) = load_synced_at(root, &[])?;
    // Every card is checked before any is written, so a refused one leaves the board as it was.
    let mut placed: Vec<(&BoardCard, PathBuf, Option<String>)> = Vec::new();
    for card in cards {
        let rel = &card.rel_parts;
        let target = locate(root, &data, rel)?;
        guard_frozen(rel, &target.node_type)?;
        if target.ghost {
            return Err(FmError::GhostNotAllowed { op: "place on the board".to_string() });
        }
        if type_base(&target.node_type) != "WKB" {
            return Err(FmError::InvalidPath { rel_parts: rel.clone(), reason: "only workboards are on the board".to_string() });
        }
        // Same precedence as the tree: the info.json status wins over the yaml one.
        let yaml_status = get_node(&data, rel).and_then(|n| n.get("status")).and_then(Value::as_str).map(str::to_string);
        let status = read_info(&target.abs).as_ref().and_then(|i| field_of(i, &["status"])).map(str::to_string).or(yaml_status);
        placed.push((card, target.abs, status));
    }
    for (card, abs, status) in placed {
        write_info(
            &abs,
            &[
                (&["board"], "board", Some(card.column.as_str().into())),
                (&["boardOrder"], "boardOrder", Some(Value::Number(u64::from(card.order).into()))),
                (&["boardStatus"], "boardStatus", status.map(Value::from)),
            ],
        )?;
    }
    if synced {
        tree_store::save(root, &data)?;
    }
    Ok(finish(root, &data, synced, None, Vec::new(), cards.iter().map(|c| c.rel_parts.clone()).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EditableType, WkbStatus};
    use crate::mutations::set_node_type;
    use crate::test_support::*;

    fn card(rel_parts: &[String], column: BoardColumn, order: u32) -> BoardCard {
        BoardCard { rel_parts: rel_parts.to_vec(), column, order }
    }

    #[test]
    fn a_card_keeps_its_column_its_order_and_the_status_it_was_placed_with() {
        let dir = academic("board");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(set_board(dir.path(), &[card(&p, BoardColumn::Todo, 0)]).is_err(), "not a workboard yet");
        set_node_type(dir.path(), &p, EditableType::Wkb, Some(WkbStatus::Hold)).unwrap();
        let m = set_board(dir.path(), &[card(&p, BoardColumn::Backlog, 3)]).unwrap();
        assert_eq!(m.select, None);
        let info = std::fs::read_to_string(dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json")).unwrap();
        assert!(info.contains("\"board\": \"backlog\"") && info.contains("\"boardOrder\": 3") && info.contains("\"boardStatus\": \"hold\""), "{info}");
        assert!(info.contains("\"status\": \"hold\""), "the status is left as it was: {info}");
    }

    #[test]
    fn a_refused_card_leaves_the_others_unwritten() {
        let dir = academic("board-refused");
        let bio = rel(&["100.ACADEMIC", "02.Bio"]);
        set_node_type(dir.path(), &bio, EditableType::Wkb, None).unwrap();
        let ghost = rel(&["100.ACADEMIC", "03.Ghost"]);
        assert!(set_board(dir.path(), &[card(&bio, BoardColumn::Done, 0), card(&ghost, BoardColumn::Done, 1)]).is_err());
        let info = std::fs::read_to_string(dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json")).unwrap();
        assert!(!info.contains("board"), "{info}");
    }
}
