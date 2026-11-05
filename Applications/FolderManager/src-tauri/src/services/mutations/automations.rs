// An Outcomes folder keeps the defaults its node gives every new workboard: output folder, status and owner.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use super::{editable_target, finish, load_synced_at, parent_of, Mutation};
use crate::domain::config::OWNER_KEYS;
use crate::domain::error::{FmError, R};
use crate::domain::tree_types::{child_entries, type_base};
use crate::infra::fs_safe::{io_err, StorageLock};
use crate::infra::storage::info::{field_of, read_info, write_info};
use crate::services::tree::tree_node::parse_status;

pub const DEFAULT_OUTPUT: &str = "defaultOutput";
pub const DEFAULT_STATUS: &str = "defaultStatus";
pub const DEFAULT_OWNER: &str = "defaultOwner";
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
                return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the output must be a folder inside the workboard".to_string() });
            }
        }
        DEFAULT_OWNER => {}
        _ => return Err(FmError::Internal { message: format!("unknown automation {key}") }),
    }
    write_info(&target.abs, &[(&[key][..], key, value.map(Value::from))])?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new(), vec![rel.to_vec()]))
}

// Fills a new workboard from its node's Outcomes defaults; an unset status keeps the in-progress it was created with.
pub(super) fn apply_defaults(parent_dir: &Path, parent_yaml: &Mapping, workboard: &Path) -> R<()> {
    let Some((name, _)) = child_entries(parent_yaml).find(|(_, c)| type_base(c.get("type").and_then(Value::as_str).unwrap_or("")) == "OUT") else {
        return Ok(());
    };
    let Some(info) = read_info(&parent_dir.join(name)) else { return Ok(()) };
    let field = |key: &str| field_of(&info, &[key]).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
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
        let dir = workboard.join(&output);
        fs::create_dir_all(&dir).map_err(|e| io_err("create output folder", &dir, &e))?;
        updates.push((&["output"], "output", Some(output.into())));
    }
    if updates.is_empty() {
        return Ok(());
    }
    write_info(workboard, &updates)
}
