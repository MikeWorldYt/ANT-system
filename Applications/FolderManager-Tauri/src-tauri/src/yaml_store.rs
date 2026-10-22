// Tree.yaml access: load, save, sync with the folders on disk and edit entries by path.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{type_base, META_KEYS};
use crate::config::{
    CORE_FOLDER, LOCKED_TYPES, MAX_DEPTH, SETTINGS_FOLDER, SPECIAL_TYPES, WKB_IDENTIFIERS,
    YAML_FILENAME,
};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, write_atomic};
use crate::names::list_subdirs;

pub fn yaml_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(CORE_FOLDER).join(YAML_FILENAME)
}

pub fn load(root: &Path) -> R<Mapping> {
    let path = yaml_path(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Mapping::new()),
        Err(e) => return Err(io_err("read Tree.yaml", &path, &e)),
    };
    let value: Value = serde_yaml::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| {
        FmError::YamlParse { message: e.to_string(), line: e.location().map(|l| l.line()) }
    })?;
    match value {
        Value::Null => Ok(Mapping::new()),
        Value::Mapping(m) => normalize_mapping(m),
        _ => Err(parse_error("top level must be a mapping")),
    }
}

pub fn save(root: &Path, data: &Mapping) -> R<()> {
    let path = yaml_path(root);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| io_err("create folder", dir, &e))?;
    }
    let text = serde_yaml::to_string(&Value::Mapping(sort_mapping(data)))
        .map_err(|e| FmError::Internal { message: e.to_string() })?;
    write_atomic(&path, text.as_bytes())
}

pub fn infer_type(name: &str, depth: usize) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if WKB_IDENTIFIERS.iter().any(|p| basename.starts_with(p)) {
        return "WKB".to_string();
    }
    match SPECIAL_TYPES.iter().find(|(prefix, _)| basename.starts_with(prefix)) {
        Some((_, folder_type)) => (*folder_type).to_string(),
        None => format!("NODE-L{}", depth + 1),
    }
}

pub fn is_locked_type(t: &str) -> bool {
    LOCKED_TYPES.contains(&type_base(t))
}

// Disk dirs missing from the yaml are added; yaml-only keys stay as ghosts.
pub fn sync(root: &Path, data: &mut Mapping) -> R<bool> {
    sync_level(root, data, 0)
}

pub fn get_node<'a>(data: &'a Mapping, rel: &[String]) -> Option<&'a Mapping> {
    let mut node = data;
    for part in rel {
        node = node.get(part.as_str())?.as_mapping()?;
    }
    Some(node)
}

pub fn get_node_mut<'a>(data: &'a mut Mapping, rel: &[String]) -> Option<&'a mut Mapping> {
    let mut node = data;
    for part in rel {
        node = node.get_mut(part.as_str())?.as_mapping_mut()?;
    }
    Some(node)
}

pub fn get_type(data: &Mapping, rel: &[String]) -> String {
    get_node(data, rel)
        .and_then(|m| m.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub fn rename_key(data: &mut Mapping, parent: &[String], old: &str, new: &str) -> bool {
    let Some(node) = get_node_mut(data, parent) else { return false };
    match node.remove(old) {
        Some(entry) => {
            node.insert(Value::String(new.to_string()), entry);
            true
        }
        None => false,
    }
}

pub fn delete_key(data: &mut Mapping, parent: &[String], key: &str) -> bool {
    get_node_mut(data, parent).is_some_and(|node| node.remove(key).is_some())
}

fn sync_level(dir: &Path, node: &mut Mapping, depth: usize) -> R<bool> {
    if depth >= MAX_DEPTH {
        return Ok(false);
    }
    let mut changed = false;
    for name in list_subdirs(dir)? {
        // A folder named like a meta key would overwrite the node's own type or status.
        if META_KEYS.contains(&name.as_str()) {
            continue;
        }
        if !matches!(node.get(name.as_str()), Some(Value::Mapping(_))) {
            let mut entry = Mapping::new();
            entry.insert("type".into(), infer_type(&name, depth).into());
            node.insert(Value::String(name.clone()), Value::Mapping(entry));
            changed = true;
        }
        if let Some(Value::Mapping(child)) = node.get_mut(name.as_str()) {
            changed |= sync_level(&dir.join(&name), child, depth + 1)?;
        }
    }
    Ok(changed)
}

fn parse_error(message: &str) -> FmError {
    FmError::YamlParse { message: message.to_string(), line: None }
}

fn normalize_mapping(m: Mapping) -> R<Mapping> {
    let mut out = Mapping::new();
    for (k, v) in m {
        let key = match k {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => return Err(parse_error("unsupported mapping key")),
        };
        out.insert(Value::String(key), normalize_value(v)?);
    }
    Ok(out)
}

fn normalize_value(v: Value) -> R<Value> {
    match v {
        Value::Mapping(m) => Ok(Value::Mapping(normalize_mapping(m)?)),
        Value::Sequence(items) => {
            Ok(Value::Sequence(items.into_iter().map(normalize_value).collect::<R<_>>()?))
        }
        other => Ok(other),
    }
}

// Byte order of UTF-8 equals PyYAML's code point order for sort_keys=True.
fn sort_mapping(m: &Mapping) -> Mapping {
    let mut entries: Vec<(&Value, &Value)> = m.iter().collect();
    entries.sort_by(|a, b| a.0.as_str().cmp(&b.0.as_str()));
    entries.into_iter().map(|(k, v)| (k.clone(), sort_value(v))).collect()
}

fn sort_value(v: &Value) -> Value {
    match v {
        Value::Mapping(m) => Value::Mapping(sort_mapping(m)),
        Value::Sequence(items) => Value::Sequence(items.iter().map(sort_value).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("fm-{tag}-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }

        pub fn mkdir(&self, rel: &str) {
            std::fs::create_dir_all(self.0.join(rel)).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::TempDir;
    use super::*;

    fn parse(s: &str) -> Mapping {
        serde_yaml::from_str(s).unwrap()
    }

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn load_missing_file_gives_empty_mapping() {
        let dir = TempDir::new("load-missing");
        assert!(load(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn save_then_load_round_trips_with_sorted_keys() {
        let dir = TempDir::new("roundtrip");
        let data = parse("zeta:\n  type: NODE-L1\n  b: {type: X}\n  a: {type: Y}\nalpha:\n  type: REV\n");
        save(dir.path(), &data).unwrap();
        let text = fs::read_to_string(yaml_path(dir.path())).unwrap();
        let pos = |needle: &str| text.find(needle).unwrap();
        assert!(pos("alpha:") < pos("zeta:"));
        assert!(pos("  a:") < pos("  b:") && pos("  b:") < pos("  type: NODE-L1"));
        let loaded = load(dir.path()).unwrap();
        assert_eq!(get_type(&loaded, &rel(&["zeta", "a"])), "Y");
        assert_eq!(get_type(&loaded, &rel(&["alpha"])), "REV");
        assert!(!yaml_path(dir.path()).with_file_name(".Tree.yaml.tmp").exists());
    }

    #[test]
    fn load_normalises_non_string_keys() {
        let dir = TempDir::new("keys");
        dir.mkdir("999.Settings/00.Core");
        fs::write(yaml_path(dir.path()), "2024:\n  type: NODE-L1\ntrue:\n  type: X\n").unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(get_type(&loaded, &rel(&["2024"])), "NODE-L1");
        assert_eq!(get_type(&loaded, &rel(&["true"])), "X");
    }

    #[test]
    fn invalid_yaml_is_a_parse_error_and_leaves_the_file_alone() {
        let dir = TempDir::new("invalid");
        dir.mkdir("999.Settings/00.Core");
        let path = yaml_path(dir.path());
        let broken = "a: [unclosed\nb: 1\n";
        fs::write(&path, broken).unwrap();
        assert!(matches!(load(dir.path()), Err(FmError::YamlParse { .. })));
        assert_eq!(fs::read_to_string(&path).unwrap(), broken);
    }

    #[test]
    fn non_mapping_top_level_is_a_parse_error() {
        let dir = TempDir::new("toplevel");
        dir.mkdir("999.Settings/00.Core");
        fs::write(yaml_path(dir.path()), "- a\n- b\n").unwrap();
        assert!(matches!(load(dir.path()), Err(FmError::YamlParse { .. })));
    }

    #[test]
    fn sync_adds_new_dirs_keeps_ghosts_and_settles() {
        let dir = TempDir::new("sync");
        dir.mkdir("01.Math/PJ.Demo");
        dir.mkdir("01.Math/PJ.Demo/deep/deeper");
        dir.mkdir("90.Resources");
        dir.mkdir(".ant-trash/x");
        let mut data = parse("ZZ.Ghost:\n  type: NODE-L1\n01.Math: null\n");
        assert!(sync(dir.path(), &mut data).unwrap());
        assert_eq!(get_type(&data, &rel(&["ZZ.Ghost"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["01.Math"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["01.Math", "PJ.Demo"])), "WKB");
        assert_eq!(get_type(&data, &rel(&["90.Resources"])), "RES");
        assert_eq!(get_type(&data, &rel(&["01.Math", "PJ.Demo", "deep"])), "NODE-L3");
        assert!(get_node(&data, &rel(&["01.Math", "PJ.Demo", "deep", "deeper"])).is_none());
        assert!(data.get(".ant-trash").is_none());
        assert!(!sync(dir.path(), &mut data).unwrap());
    }

    #[test]
    fn infer_type_follows_prefixes_and_depth() {
        assert_eq!(infer_type("PJ.Thesis", 1), "WKB");
        assert_eq!(infer_type("PR.X", 0), "WKB");
        assert_eq!(infer_type("HW.X", 2), "WKB");
        assert_eq!(infer_type("90.Resources", 1), "RES");
        assert_eq!(infer_type("97.Templates", 1), "TPL");
        assert_eq!(infer_type("98.Review", 2), "REV");
        assert_eq!(infer_type("01.Math", 0), "NODE-L1");
        assert_eq!(infer_type("a/b/02.X", 2), "NODE-L3");
    }

    #[test]
    fn locked_types_are_rev_res_tpl_cfg() {
        for t in ["REV", "REV-L3", "RES", "TPL-L1", "CFG"] {
            assert!(is_locked_type(t), "{t}");
        }
        for t in ["NODE-L1", "WKB", "PORTF", "", "REVX"] {
            assert!(!is_locked_type(t), "{t}");
        }
    }

    #[test]
    fn rename_and_delete_key_act_on_the_parent_mapping() {
        let mut data = parse("p:\n  a: {type: X}\n  b: {type: Y}\n");
        assert!(rename_key(&mut data, &rel(&["p"]), "a", "c"));
        assert_eq!(get_type(&data, &rel(&["p", "c"])), "X");
        assert!(get_node(&data, &rel(&["p", "a"])).is_none());
        assert!(!rename_key(&mut data, &rel(&["p"]), "nope", "d"));
        assert!(!rename_key(&mut data, &rel(&["missing"]), "a", "d"));
        assert!(delete_key(&mut data, &rel(&["p"]), "b"));
        assert!(!delete_key(&mut data, &rel(&["p"]), "b"));
    }
}
