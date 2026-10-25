// Reads node types stored in the tree ("NODE-L2", "REV-L3", "CFG") and the entries that name child folders.
use serde_yaml::Mapping;

const META_KEYS: [&str; 2] = ["type", "status"];

pub fn type_base(t: &str) -> &str {
    t.split('-').next().unwrap_or("")
}

pub fn node_type(m: &Mapping) -> &str {
    type_base(m.get("type").and_then(|v| v.as_str()).unwrap_or(""))
}

pub fn child_entries(m: &Mapping) -> impl Iterator<Item = (&str, &Mapping)> {
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
    fn child_entries_skips_meta_keys_and_scalars() {
        let m: Mapping = serde_yaml::from_str("type: NODE-L2\nstatus: done\nnote: text\n01.A:\n  type: WKB\n").unwrap();
        let kids: Vec<&str> = child_entries(&m).map(|(k, _)| k).collect();
        assert_eq!(kids, vec!["01.A"]);
        assert_eq!(node_type(&m), "NODE");
        assert_eq!(type_base("CFG"), "CFG");
    }
}
