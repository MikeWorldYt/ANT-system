// An Outcomes folder keeps the defaults its node gives every new worknode; a review folder keeps which of its automations run on their own.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use super::{editable_target, finish, locate, scope, load_synced_at, parent_of, Mutation};
use crate::domain::config::{AUTO_ADD_DATE_FLAG, AUTO_GROUP_FLAG, AUTO_REDUCE_FLAG, AUTO_REMOVE_DATE_FLAG, AUTO_REMOVE_SYMBOLS_FLAG, AUTO_SORT_FLAG, FOOTPRINT_FILENAME, OWNER_KEYS, TEMPLATES_ROOT, WKB_IDENTIFIERS, DATA_DIR};
use crate::domain::error::{FmError, R};
use crate::domain::names::sanitize_train_case;
use crate::domain::tree_types::{child_entries, flags_in, runs_automations, type_base};
use crate::infra::fs_safe::{io_err, validate_component, StorageLock};
use crate::infra::footprint_file::mirror_flags;
use crate::infra::storage::info::{field_of, read_info, write_info, NodeInfo};
use crate::services::tree::tree_store::{self, get_node_mut, set_flag};
use crate::services::tree::tree_node::parse_status;

pub const DEFAULT_OUTPUT: &str = "defaultOutput";
pub const DEFAULT_STATUS: &str = "defaultStatus";
pub const DEFAULT_OWNER: &str = "defaultOwner";
pub const DEFAULT_TEMPLATE: &str = "defaultTemplate";
// Stored for an explicitly empty status, so it differs from the unset in-progress default.
const NO_STATUS: &str = "none";

pub fn set_default(root: &Path, rel: &[String], key: &str, value: Option<&str>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = editable_target(root, &data, rel, "edit")?;
    if type_base(&target.node_type) != "OUT" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only an Outcomes folder has automations".to_string() });
    }
    let value = value.map(str::trim).filter(|v| !v.is_empty());
    match key {
        DEFAULT_STATUS => {
            if value.is_some_and(|v| v != NO_STATUS && parse_status(v).is_none()) {
                return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "unknown status".to_string() });
            }
        }
        DEFAULT_OUTPUT => {
            if value.is_some_and(|v| v.contains("..") || Path::new(v).is_absolute()) {
                return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the output must be a folder inside the worknode".to_string() });
            }
        }
        DEFAULT_OWNER => {}
        DEFAULT_TEMPLATE => {
            if let Some(name) = value {
                validate_component(name)?;
                if !root.join(TEMPLATES_ROOT).join(name).is_dir() {
                    return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: format!("no template named {name} in {TEMPLATES_ROOT}") });
                }
            }
        }
        _ => return Err(FmError::Internal { message: format!("unknown automation {key}") }),
    }
    write_info(&target.abs, &[(&[key][..], key, value.map(Value::from))])?;
    Ok(finish(root, &data, false, Some(rel.to_vec()), Vec::new(), scope(&synced, vec![rel.to_vec()])))
}

pub fn set_review_automation(root: &Path, rel: &[String], automation: &str, on: bool) -> R<Mutation> {
    let flag = match automation {
        "reduce" => AUTO_REDUCE_FLAG,
        "sort" => AUTO_SORT_FLAG,
        "group" => AUTO_GROUP_FLAG,
        "addDate" => AUTO_ADD_DATE_FLAG,
        "removeDate" => AUTO_REMOVE_DATE_FLAG,
        "removeSymbols" => AUTO_REMOVE_SYMBOLS_FLAG,
        _ => return Err(FmError::Internal { message: format!("unknown automation {automation}") }),
    };
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = locate(root, &data, rel)?;
    let node = get_node_mut(&mut data, rel).ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    if target.ghost || !runs_automations(type_base(&target.node_type), &flags_in(node)) {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "this folder has no automations; activate them first".to_string() });
    }
    set_flag(node, flag, on);
    let flags = flags_in(node);
    tree_store::save(root, &data)?;
    // The tree already holds the flag, so a failed mirror into dragpath.json is only reported.
    let warnings = mirror_flags(&target.abs, &flags).err().map(|e| format!("Could not update {FOOTPRINT_FILENAME}: {e}")).into_iter().collect();
    Ok(finish(root, &data, false, Some(rel.to_vec()), warnings, scope(&synced, vec![rel.to_vec()])))
}

fn outcomes_info(parent_dir: &Path, parent_yaml: &Mapping) -> Option<NodeInfo> {
    let (name, _) = child_entries(parent_yaml).find(|(_, c)| type_base(c.get("type").and_then(Value::as_str).unwrap_or("")) == "OUT")?;
    read_info(&parent_dir.join(name))
}

fn info_field(info: &NodeInfo, key: &str) -> Option<String> {
    field_of(info, &[key]).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string)
}

// The template the node's Outcomes picked, while its folder is still there.
pub(super) fn default_template(root: &Path, parent_dir: &Path, parent_yaml: &Mapping) -> Option<String> {
    let name = info_field(&outcomes_info(parent_dir, parent_yaml)?, DEFAULT_TEMPLATE)?;
    root.join(TEMPLATES_ROOT).join(&name).is_dir().then_some(name)
}

// "PR.Video-Short" makes PR worknodes; a template without a known identifier keeps the PJ default.
pub(super) fn template_identifier(template: &str) -> &'static str {
    WKB_IDENTIFIERS.iter().find(|id| template.get(..id.len()).is_some_and(|head| head.eq_ignore_ascii_case(id))).copied().unwrap_or("PJ.")
}

// A template is an unnumbered folder of ANT.TEMPLATES named by its identifier and title; its description stays in its own .ant, which is never copied.
pub fn create_template(root: &Path, identifier: &str, name: &str, description: Option<&str>) -> R<Vec<String>> {
    let id = WKB_IDENTIFIERS.iter().find(|i| i.trim_end_matches('.').eq_ignore_ascii_case(identifier.trim_end_matches('.'))).ok_or_else(|| FmError::InvalidName { name: identifier.to_string(), reason: "unknown worknode identifier".to_string() })?;
    let title = sanitize_train_case(name);
    if title.is_empty() {
        return Err(FmError::InvalidName { name: name.to_string(), reason: "name cannot be empty".to_string() });
    }
    let folder = format!("{id}{title}");
    validate_component(&folder)?;
    let _lock = StorageLock::acquire(root)?;
    let home = root.join(TEMPLATES_ROOT);
    fs::create_dir_all(&home).map_err(|e| io_err("create folder", &home, &e))?;
    let dir = home.join(&folder);
    if dir.exists() {
        return Err(FmError::AlreadyExists { name: folder });
    }
    fs::create_dir(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
    if let Some(text) = description.map(str::trim).filter(|t| !t.is_empty()) {
        write_info(&dir, &[(&["description"][..], "description", Some(text.into()))])?;
    }
    Ok(vec![TEMPLATES_ROOT.to_string(), folder])
}

// What the worknode already has, its .ant marker above all, is never overwritten.
fn copy_into(from: &Path, to: &Path) -> R<()> {
    for entry in fs::read_dir(from).map_err(|e| io_err("read template", from, &e))? {
        let entry = entry.map_err(|e| io_err("read template", from, &e))?;
        let name = entry.file_name();
        let (src, dst) = (entry.path(), to.join(&name));
        if name == DATA_DIR || dst.exists() {
            continue;
        }
        if src.is_dir() {
            fs::create_dir(&dst).map_err(|e| io_err("copy template", &dst, &e))?;
            copy_into(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| io_err("copy template", &dst, &e))?;
        }
    }
    Ok(())
}

// Fills a new worknode from its node's Outcomes defaults; an unset status keeps the in-progress it was created with.
pub(super) fn apply_defaults(root: &Path, parent_dir: &Path, parent_yaml: &Mapping, worknode: &Path) -> R<()> {
    if let Some(template) = default_template(root, parent_dir, parent_yaml) {
        copy_into(&root.join(TEMPLATES_ROOT).join(template), worknode)?;
    }
    let Some(info) = outcomes_info(parent_dir, parent_yaml) else { return Ok(()) };
    let field = |key: &str| info_field(&info, key);
    let mut updates: Vec<(&[&str], &str, Option<Value>)> = Vec::new();
    match field(DEFAULT_STATUS).as_deref() {
        Some(NO_STATUS) => updates.push((&["status"], "status", None)),
        Some(status) => updates.push((&["status"], "status", Some(status.into()))),
        None => {}
    }
    if let Some(owner) = field(DEFAULT_OWNER) {
        updates.push((&OWNER_KEYS, "owner", Some(owner.into())));
    }
    if let Some(output) = field(DEFAULT_OUTPUT) {
        let dir = worknode.join(&output);
        fs::create_dir_all(&dir).map_err(|e| io_err("create output folder", &dir, &e))?;
        updates.push((&["output"], "output", Some(output.into())));
    }
    if updates.is_empty() {
        return Ok(());
    }
    write_info(worknode, &updates)
}
