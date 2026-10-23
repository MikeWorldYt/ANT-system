// Tree.yaml access: load, save, sync with the folders on disk and edit entries by path.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{type_base, META_KEYS};
use crate::config::{
    is_hidden_dir, CORE_FOLDER, LOCKED_TYPES, MAX_DEPTH, SETTINGS_FOLDER, SPECIAL_TYPES, WKB_IDENTIFIERS,
    YAML_FILENAME,
};
use crate::db::{self, Store};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, write_atomic};
use crate::names::list_subdirs;

pub fn yaml_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(CORE_FOLDER).join(YAML_FILENAME)
}

// The db is the primary copy; Tree.yaml is re-imported only when something else changed it since our last write.
pub fn load(root: &Path) -> R<Mapping> {
    let yaml = read_yaml(root)?;
    let Some(mut store) = Store::open_existing(root)? else {
        let Some(text) = yaml else { return Ok(Mapping::new()) };
        let data = parse(&text)?;
        Store::open_or_create(root)?.replace(&data, &db::hash(&text), || Ok(()), || Ok(()))?;
        return Ok(data);
    };
    match yaml {
        Some(text) if store.yaml_hash()?.as_deref() != Some(db::hash(&text).as_str()) => {
            let data = parse(&text)?;
            store.replace(&data, &db::hash(&text), || Ok(()), || Ok(()))?;
            Ok(data)
        }
        _ => store.read(),
    }
}

pub fn save(root: &Path, data: &Mapping) -> R<()> {
    let path = yaml_path(root);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| io_err("create folder", dir, &e))?;
    }
    let sorted = sort_mapping(data);
    let mut text = String::new();
    if sorted.is_empty() {
        text.push_str("{}\n");
    } else {
        emit_mapping(&mut text, &sorted, 0)?;
    }
    let previous = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_err("read Tree.yaml", &path, &e)),
    };
    let restore = || match &previous {
        Some(bytes) => write_atomic(&path, bytes),
        None => fs::remove_file(&path).map_err(|e| io_err("remove Tree.yaml", &path, &e)),
    };
    Store::open_or_create(root)?.replace(&sorted, &db::hash(&text), || write_atomic(&path, text.as_bytes()), restore)
}

fn read_yaml(root: &Path) -> R<Option<String>> {
    let path = yaml_path(root);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err("read Tree.yaml", &path, &e)),
    }
}

fn parse(text: &str) -> R<Mapping> {
    let value: Value = serde_yaml::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| {
        FmError::YamlParse { message: e.to_string(), line: e.location().map(|l| l.line()) }
    })?;
    let mut data = match value {
        Value::Null => Mapping::new(),
        Value::Mapping(m) => normalize_mapping(m)?,
        _ => return Err(parse_error("top level must be a mapping")),
    };
    // Older tools list every folder, including the ones the app owns.
    data.retain(|k, _| !k.as_str().is_some_and(is_hidden_dir));
    Ok(data)
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

// Only child entries (mappings) move or go, so a node's own type or status can never be touched.
pub fn rename_key(data: &mut Mapping, parent: &[String], old: &str, new: &str) -> bool {
    let Some(node) = get_node_mut(data, parent) else { return false };
    if META_KEYS.contains(&new) || !matches!(node.get(old), Some(Value::Mapping(_))) {
        return false;
    }
    match node.remove(old) {
        Some(entry) => {
            node.insert(Value::String(new.to_string()), entry);
            true
        }
        None => false,
    }
}

pub fn delete_key(data: &mut Mapping, parent: &[String], key: &str) -> bool {
    let Some(node) = get_node_mut(data, parent) else { return false };
    matches!(node.get(key), Some(Value::Mapping(_))) && node.remove(key).is_some()
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

// Block mappings are written here so keys can be quoted for PyYAML; other values reuse serde_yaml's text.
fn emit_mapping(out: &mut String, m: &Mapping, indent: usize) -> R<()> {
    let pad = " ".repeat(indent);
    for (k, v) in m {
        let key = inline_scalar(k)?;
        match v {
            Value::Mapping(child) if !child.is_empty() => {
                out.push_str(&format!("{pad}{key}:\n"));
                emit_mapping(out, child, indent + 2)?;
            }
            Value::Sequence(items) if !items.is_empty() => emit_block(out, &pad, &key, v)?,
            Value::Tagged(_) => emit_block(out, &pad, &key, v)?,
            Value::String(s) if s.contains('\n') => emit_block(out, &pad, &key, v)?,
            _ => out.push_str(&format!("{pad}{key}: {}\n", inline_scalar(v)?)),
        }
    }
    Ok(())
}

// serde_yaml writes `k:` first for a one-entry mapping, so its remaining lines are reused under our key.
fn emit_block(out: &mut String, pad: &str, key: &str, v: &Value) -> R<()> {
    let mut wrapper = Mapping::new();
    wrapper.insert(Value::String("k".to_string()), v.clone());
    let text = yaml_text(&Value::Mapping(wrapper))?;
    let rest = text.strip_prefix("k:").ok_or_else(|| internal("unexpected YAML output"))?;
    for (i, line) in rest.lines().enumerate() {
        match (i, line.is_empty()) {
            (0, _) => out.push_str(&format!("{pad}{key}:{line}\n")),
            (_, true) => out.push('\n'),
            _ => out.push_str(&format!("{pad}{line}\n")),
        }
    }
    Ok(())
}

fn inline_scalar(v: &Value) -> R<String> {
    if let Value::String(s) = v {
        if !s.contains('\n') && yaml11_ambiguous(s) {
            return Ok(format!("'{}'", s.replace('\'', "''")));
        }
    }
    let text = yaml_text(v)?;
    let line = text.strip_suffix('\n').unwrap_or(&text);
    if line.contains('\n') {
        return Err(internal("a key or value does not fit on one line"));
    }
    Ok(line.to_string())
}

fn yaml_text(v: &Value) -> R<String> {
    serde_yaml::to_string(v).map_err(|e| internal(&e.to_string()))
}

fn internal(message: &str) -> FmError {
    FmError::Internal { message: message.to_string() }
}

// PyYAML (YAML 1.1) reads these plain scalars as bools, nulls, numbers or dates, unlike serde_yaml.
fn yaml11_ambiguous(s: &str) -> bool {
    const WORDS: &[&str] = &["y", "n", "yes", "no", "on", "off", "true", "false", "null", "~", "<<", "="];
    if s.is_empty() || WORDS.iter().any(|w| w.eq_ignore_ascii_case(s)) {
        return true;
    }
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    let radix = ["0x", "0X", "0b", "0B", "0o", "0O"].iter().any(|p| unsigned.starts_with(p));
    let numeric_start = s.starts_with(|c: char| c.is_ascii_digit() || "+-.".contains(c));
    numeric_start
        && (radix && unsigned[2..].chars().all(|c| c.is_ascii_hexdigit() || c == '_')
            || s.chars().all(|c| c.is_ascii_digit() || "_+-.:eE".contains(c)))
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
        let left: Vec<_> = fs::read_dir(yaml_path(dir.path()).parent().unwrap()).unwrap().collect();
        assert_eq!(left.len(), 1);
    }

    #[test]
    fn save_quotes_yaml11_tokens_so_pyyaml_reads_strings() {
        let dir = TempDir::new("quote");
        let data = parse(
            "'yes': {type: NODE-L1}\n'Y': {type: X}\n'on': {type: X}\n'OFF': {type: X}\n'1_000': {type: X}\n\
             '2024-01-15': {type: X}\n'0x1F': {type: X}\n'01.10': {type: X}\n01.Math:\n  type: NODE-L1\n  status: 'no'\n\
             note: |\n  line one\n  line two\nlist: [a, 'off']\nempty: {}\n",
        );
        save(dir.path(), &data).unwrap();
        let text = fs::read_to_string(yaml_path(dir.path())).unwrap();
        for quoted in ["'yes':", "'Y':", "'on':", "'OFF':", "'1_000':", "'2024-01-15':", "'0x1F':", "'01.10':", "status: 'no'"] {
            assert!(text.contains(quoted), "{quoted} missing in:\n{text}");
        }
        assert!(text.contains("\n01.Math:\n") && text.contains("empty: {}\n"), "{text}");
        assert_eq!(load(dir.path()).unwrap(), data);
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

    #[test]
    fn rename_and_delete_key_never_touch_meta_values() {
        let mut data = parse("p:\n  type: NODE-L1\n  status: done\n  a: {type: X}\n");
        assert!(!delete_key(&mut data, &rel(&["p"]), "type"));
        assert!(!rename_key(&mut data, &rel(&["p"]), "status", "x"));
        assert!(!rename_key(&mut data, &rel(&["p"]), "a", "type"));
        assert_eq!(get_type(&data, &rel(&["p"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["p", "a"])), "X");
        assert!(get_node(&data, &rel(&["p"])).unwrap().get("status").is_some());
    }

    const RICH: &str = "version: 3\nzeta:\n  type: NODE-L1\n  status: done\n  note: hello\n  a: {type: Y}\n  b: {}\nalpha:\n  type: REV\n";

    #[test]
    fn save_creates_a_hidden_db_and_the_yaml_mirror() {
        use std::os::windows::fs::MetadataExt;
        let dir = TempDir::new("db-created");
        save(dir.path(), &parse(RICH)).unwrap();
        let data_dir = dir.path().join(".ant");
        assert!(data_dir.join("ant.db").is_file() && yaml_path(dir.path()).is_file());
        assert_ne!(fs::metadata(&data_dir).unwrap().file_attributes() & 0x2, 0, ".ant is not hidden");
    }

    #[test]
    fn db_round_trips_extra_keys_and_root_entries() {
        let dir = TempDir::new("db-roundtrip");
        let data = parse(RICH);
        save(dir.path(), &data).unwrap();
        assert_eq!(load(dir.path()).unwrap(), data);
    }

    #[test]
    fn an_external_yaml_edit_replaces_the_db_copy() {
        let dir = TempDir::new("db-external");
        save(dir.path(), &parse("one: {type: NODE-L1}\n")).unwrap();
        fs::write(yaml_path(dir.path()), "two: {type: NODE-L1}\n").unwrap();
        let loaded = load(dir.path()).unwrap();
        assert!(loaded.contains_key("two") && !loaded.contains_key("one"));
        assert_eq!(load(dir.path()).unwrap(), loaded);
    }

    #[test]
    fn a_missing_yaml_is_served_from_the_db_and_rewritten_on_save() {
        let dir = TempDir::new("db-yaml-gone");
        let data = parse(RICH);
        save(dir.path(), &data).unwrap();
        fs::remove_file(yaml_path(dir.path())).unwrap();
        assert_eq!(load(dir.path()).unwrap(), data);
        save(dir.path(), &data).unwrap();
        assert!(yaml_path(dir.path()).is_file());
    }

    #[test]
    fn first_load_imports_the_yaml_and_drops_app_folders() {
        let dir = TempDir::new("db-import");
        dir.mkdir("999.Settings/00.Core");
        fs::write(yaml_path(dir.path()), "01.A: {type: NODE-L1}\n.ant: {type: NODE-L1}\n.ANT-TRASH: {type: NODE-L1}\n").unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(dir.path().join(".ant/ant.db").is_file());
    }

    #[test]
    fn a_failed_yaml_write_leaves_the_db_unchanged() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TempDir::new("db-failed-write");
        let before = parse("one: {type: NODE-L1}\n");
        save(dir.path(), &before).unwrap();
        let _held = fs::OpenOptions::new().read(true).share_mode(0x1 | 0x2).open(yaml_path(dir.path())).unwrap();
        assert!(save(dir.path(), &parse("two: {type: NODE-L1}\n")).is_err());
        assert_eq!(load(dir.path()).unwrap(), before);
    }
}
