// Reads node types stored in the tree ("NODE-L2", "REV-L3", "CFG") and the entries that name child folders.
use serde_yaml::Mapping;

pub const META_KEYS: [&str; 4] = ["type", "status", "uid", "flags"];

pub fn type_base(t: &str) -> &str {
    t.split('-').next().unwrap_or("")
}

pub fn type_level(t: &str) -> Option<&str> {
    t.split('-').nth(1).filter(|s| !s.is_empty())
}

pub fn child_entries<'a>(m: &'a Mapping) -> impl Iterator<Item = (&'a str, &'a Mapping)> + 'a {
    m.iter().filter_map(|(k, v)| {
        let key = k.as_str()?;
        if META_KEYS.contains(&key) {
            return None;
        }
        v.as_mapping().map(|child| (key, child))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_base_takes_text_before_first_dash() {
        assert_eq!(type_base("REV-L3"), "REV");
        assert_eq!(type_base("CFG"), "CFG");
        assert_eq!(type_base(""), "");
    }

    #[test]
    fn type_level_is_none_without_a_level() {
        assert_eq!(type_level("NODE-L2"), Some("L2"));
        assert_eq!(type_level("NODE"), None);
        assert_eq!(type_level("NODE-"), None);
    }

    #[test]
    fn child_entries_skips_meta_keys_and_scalars() {
        let m: Mapping = serde_yaml::from_str(
            "type: NODE-L2\nstatus: done\nnote: text\n01.A:\n  type: WKB\n",
        )
        .unwrap();
        let kids: Vec<&str> = child_entries(&m).map(|(k, _)| k).collect();
        assert_eq!(kids, vec!["01.A"]);
    }
}
