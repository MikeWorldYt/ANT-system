// Names of the 998.Review shortcuts, derived from the owner path so review links keep the owner's numbering.
use crate::error::{FmError, R};

pub fn review_shortcut_name(owner: &[String], rev_folder: &str) -> R<String> {
    match owner {
        [] if rev_folder.is_empty() => Err(FmError::Internal {
            message: "A top-level review folder needs its own name".to_string(),
        }),
        [] => Ok(rev_folder.to_string()),
        [only] => Ok(only.clone()),
        [first, second] => Ok(format!("{}{}", first_char(first, owner)?, second)),
        [first, second, third] => {
            let letter = first_char(first, owner)?;
            let number = second.split('.').next().unwrap_or("");
            Ok(format!("{letter}{number}.{third}"))
        }
        _ => Err(FmError::InvalidPath {
            rel_parts: owner.to_vec(),
            reason: "No review shortcut below level 3".to_string(),
        }),
    }
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
        assert_eq!(
            review_shortcut_name(&parts(&["400.ACADEMIC"]), "").unwrap(),
            "400.ACADEMIC"
        );
        assert_eq!(
            review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math"]), "").unwrap(),
            "401.Math"
        );
        assert_eq!(
            review_shortcut_name(&parts(&["400.ACADEMIC", "01.Math", "PJ.Tesis"]), "").unwrap(),
            "401.PJ.Tesis"
        );
        assert_eq!(review_shortcut_name(&parts(&[]), "998.Top").unwrap(), "998.Top");
    }

    #[test]
    fn review_shortcut_name_rejects_invalid_owners() {
        assert!(review_shortcut_name(&parts(&[]), "").is_err());
        assert!(review_shortcut_name(&parts(&["a", "b", "c", "d"]), "").is_err());
        assert!(review_shortcut_name(&parts(&["", "01.Math"]), "").is_err());
    }
}
