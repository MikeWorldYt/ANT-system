// Names of the 998.Review shortcuts, derived from the owner path so review links keep the owner's numbering.
use crate::domain::error::{FmError, R};
use crate::domain::names::editable_part;

// Names sort like the tree: the category, then one two-digit slot per level below the second, "00" where the owner is shallower.
pub fn review_shortcut_name(owner: &[String], levels: usize) -> R<String> {
    if owner.len() > levels {
        return Err(FmError::InvalidPath {
            rel_parts: owner.to_vec(),
            reason: "No review shortcut below the storage's levels".to_string(),
        });
    }
    match owner {
        [] => Err(FmError::Internal {
            message: "A review shortcut needs an owner folder".to_string(),
        }),
        [only] => Ok(only.clone()),
        [first, second, rest @ ..] => {
            let mut name = format!("{}{}", first_char(first, owner)?, slot(second));
            for i in 0..levels.saturating_sub(2) {
                name.push('.');
                name.push_str(rest.get(i).map_or("00", |part| slot(part)));
            }
            name.push('.');
            name.push_str(editable_part(&owner[owner.len() - 1]));
            Ok(name)
        }
    }
}

fn slot(folder: &str) -> &str {
    folder.split('.').next().unwrap_or("")
}

fn first_char(folder: &str, owner: &[String]) -> R<char> {
    folder.chars().next().ok_or_else(|| FmError::InvalidPath {
        rel_parts: owner.to_vec(),
        reason: "Empty folder name in review owner path".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn review_shortcut_name_follows_owner_depth() {
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC"]), 3).unwrap(), "400.ACADEMIC");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math"]), 3).unwrap(), "401.00.Math");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "01.Algebra"]), 3).unwrap(), "401.01.Algebra");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "PJ.Tesis"]), 3).unwrap(), "401.PJ.Tesis");
    }

    #[test]
    fn review_shortcut_name_adds_a_slot_for_each_extra_level() {
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math"]), 4).unwrap(), "401.00.00.Math");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "01.Algebra"]), 4).unwrap(), "401.01.00.Algebra");
        assert_eq!(review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "01.Algebra", "01.Sub"]), 4).unwrap(), "401.01.01.Sub");
    }

    #[test]
    fn review_shortcut_name_rejects_invalid_owners() {
        assert!(review_shortcut_name(&parts(&[]), 3).is_err());
        assert!(review_shortcut_name(&parts(&["a", "b", "c", "d"]), 3).is_err());
        assert!(review_shortcut_name(&parts(&["", "01.Math"]), 3).is_err());
    }
}
