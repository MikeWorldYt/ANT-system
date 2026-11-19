// The program tools: the storage's settings, its extensions, and which listed storage this server works on.
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{json, Value};

use super::gate::WRITES_KEY;
use super::protocol::{RpcError, INVALID_PARAMS};
use super::schema::{object, output, strings};
use super::tools::args;
use super::Server;
use crate::commands::settings::check;
use crate::domain::error::{FmError, R};
use crate::infra::storage::db::Store;
use crate::infra::storage::{known, location, rename};
use crate::services::extensions;
use crate::services::watch::local_data_dir;

type Schemas = fn() -> (Value, Value);

const TOOLS: [(&str, &str, &str, Schemas); 8] = [
    ("get_settings", "Settings", "The open storage's Folder Manager settings, key by key; read ant://guide/settings for what each means.", get_settings),
    ("set_setting", "Change setting", "Changes one setting of the open storage; the user confirms in the window. The AI writes switch is the user's alone.", set_setting),
    ("list_extensions", "Extensions", "The extensions installed in the open storage, with their state.", none_read),
    ("list_extension_library", "Extension library", "The extensions the online library offers, and which are installed.", none_read),
    ("install_extension", "Install extension", "Installs an extension from the online library; the user confirms in the window.", by_id),
    ("set_extension_enabled", "Turn extension on or off", "Turns an installed extension on or off; the user confirms in the window.", toggle_id),
    ("remove_extension", "Remove extension", "Removes an installed extension; the user confirms in the window.", by_id),
    ("list_storages", "Storages", "The storages Folder Manager knows, which one this server works on, and whether each is still on disk.", none_read),
];
const SWITCH: (&str, &str, &str, Schemas) = ("switch_storage", "Switch storage", "Makes this server work on another storage from list_storages; the open window keeps its own.", switch_storage);

pub(super) fn has(name: &str) -> bool {
    name == SWITCH.0 || TOOLS.iter().any(|t| t.0 == name)
}

pub(super) fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .chain(std::iter::once(&SWITCH))
        .map(|(name, title, description, schemas)| {
            let (input, out) = schemas();
            let read = name.starts_with("get_") || name.starts_with("list_");
            let annotations = if read {
                json!({ "title": title, "readOnlyHint": true, "openWorldHint": *name == "list_extension_library" })
            } else {
                json!({ "title": title, "readOnlyHint": false, "destructiveHint": *name == "remove_extension", "idempotentHint": *name != "install_extension", "openWorldHint": *name == "install_extension" })
            };
            json!({ "name": name, "title": title, "description": description, "inputSchema": input, "outputSchema": out, "annotations": annotations })
        })
        .collect()
}

fn any() -> Value {
    output(json!({}), &[])
}

fn none_read() -> (Value, Value) {
    (object(json!({}), &[]), any())
}

fn get_settings() -> (Value, Value) {
    (object(json!({}), &[]), output(json!({ "settings": { "type": "object" } }), &["settings"]))
}

fn set_setting() -> (Value, Value) {
    let input = object(json!({ "key": { "type": "string", "minLength": 1 }, "value": { "type": "string", "description": "Booleans as \"true\" or \"false\"" } }), &["key", "value"]);
    (input, output(json!({ "changed": { "type": "string" }, "warnings": strings() }), &["changed"]))
}

fn by_id() -> (Value, Value) {
    (object(json!({ "id": { "type": "string", "minLength": 1 } }), &["id"]), any())
}

fn toggle_id() -> (Value, Value) {
    (object(json!({ "id": { "type": "string", "minLength": 1 }, "enabled": { "type": "boolean" } }), &["id", "enabled"]), any())
}

fn switch_storage() -> (Value, Value) {
    (object(json!({ "path": { "type": "string", "minLength": 1, "description": "A path from list_storages" } }), &["path"]), any())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingArgs {
    key: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdArgs {
    id: String,
    enabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}

fn known_list() -> R<Vec<PathBuf>> {
    Ok(known::read(&known::file(&local_data_dir()?)))
}

impl Server {
    fn confirmed(&self, title: &str, item: String, f: impl FnOnce(&std::path::Path) -> R<Value>) -> R<Value> {
        let storage = self.storage()?;
        let root = storage.root()?;
        self.allow(&root, title, true, vec![item])?;
        let _guard = storage.guard();
        f(&root)
    }
}

pub(super) fn call(server: &Server, name: &str, arguments: &Value) -> Result<R<Value>, RpcError> {
    Ok(match name {
        "get_settings" => {
            args::<NoArgs>(name, arguments)?;
            server.read(|_, root| Ok(json!({ "settings": Store::open_existing(root)?.map(|s| s.settings()).transpose()?.unwrap_or_default() })))
        }
        "set_setting" => {
            let a: SettingArgs = args(name, arguments)?;
            if a.key == WRITES_KEY {
                return Err((INVALID_PARAMS, "Only the user can change whether AI agents may change this storage, in Settings > Advanced".to_string()));
            }
            if let Err(e) = check(&a.key, &a.value) {
                return Err((INVALID_PARAMS, e.to_string()));
            }
            server.confirmed("Change setting", format!("{} = {}", a.key, a.value), |root| {
                Store::open_or_create(root)?.set_setting(&a.key, &a.value)?;
                Ok(json!({ "changed": format!("Set {} to {:?}; the open window applies it on its next start or when the setting is shown again", a.key, a.value), "warnings": [] }))
            })
        }
        "list_extensions" => {
            args::<NoArgs>(name, arguments)?;
            server.read(|_, root| Ok(json!({ "extensions": extensions::list(root)? })))
        }
        "list_extension_library" => {
            args::<NoArgs>(name, arguments)?;
            server.read(|_, root| Ok(json!({ "extensions": extensions::entries(root, extensions::fetch_registry()?) })))
        }
        "install_extension" => {
            let a: IdArgs = args(name, arguments)?;
            server.confirmed("Install extension", a.id.clone(), |root| {
                extensions::check_id(&a.id)?;
                let item = extensions::fetch_registry()?.into_iter().find(|i| i.id == a.id).ok_or_else(|| FmError::Internal { message: format!("{} is not in the extension library", a.id) })?;
                let bytes = extensions::archive(&item)?;
                let staged = extensions::stage_registry(root, item, &bytes)?;
                Ok(json!({ "installed": extensions::from_registry(root, &staged)? }))
            })
        }
        "set_extension_enabled" => {
            let a: IdArgs = args(name, arguments)?;
            let on = a.enabled.ok_or((INVALID_PARAMS, format!("Invalid arguments for {name}: missing field `enabled`")))?;
            server.confirmed("Turn extension on or off", format!("{} → {}", a.id, if on { "on" } else { "off" }), |root| {
                extensions::set_enabled(root, &a.id, on)?;
                Ok(json!({ "changed": format!("Turned {:?} {}", a.id, if on { "on" } else { "off" }) }))
            })
        }
        "remove_extension" => {
            let a: IdArgs = args(name, arguments)?;
            server.confirmed("Remove extension", a.id.clone(), |root| {
                extensions::remove(root, &a.id)?;
                Ok(json!({ "changed": format!("Removed {:?}", a.id) }))
            })
        }
        "list_storages" => {
            args::<NoArgs>(name, arguments)?;
            (|| {
                let current = server.storage().and_then(|s| s.root()).ok();
                let rows: Vec<Value> = known_list()?
                    .into_iter()
                    .map(|p| json!({ "path": p, "name": rename::folder_name(&p), "exists": crate::infra::scan::find_root(&p).is_some(), "current": current.as_deref() == Some(p.as_path()) }))
                    .collect();
                Ok(json!({ "storages": rows }))
            })()
        }
        "switch_storage" => {
            let a: PathArgs = args(name, arguments)?;
            (|| {
                let storage = server.storage()?;
                let root = location::storage_root(&location::clean_path(&a.path)?)?;
                if !known_list()?.iter().any(|p| p == &root) {
                    return Err(FmError::Internal { message: "That storage is not on Folder Manager's list; open it in the app once first".to_string() });
                }
                let _guard = storage.guard();
                storage.set_root(root.clone());
                storage.set_start(root.clone());
                Ok(json!({ "storage": root, "name": rename::folder_name(&root) }))
            })()
        }
        _ => return Err((INVALID_PARAMS, format!("Unknown tool: {name}"))),
    })
}
