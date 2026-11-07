// The read tools: each calls the service its IPC command uses and answers with structured JSON or an FmError.
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::protocol::{RpcError, INVALID_PARAMS};
use super::view::{Brief, Detail};
use super::{schema, Server};
use crate::app::storage_session::StorageInfo;
use crate::domain::error::{FmError, R};
use crate::domain::search::{parse_query, suggest_key};
use crate::services::index::disk_search;
use crate::services::tree::depth;
use crate::services::tree::level::level;
use crate::services::tree::tree_node::{build_tree, fill_children};
use crate::services::tree::tree_store;

type Schemas = fn() -> (Value, Value);

const TOOLS: [(&str, &str, &str, Schemas); 7] = [
    ("get_storage", "Storage", "The open storage: its root path, the app version and the tree depth limit.", schema::get_storage),
    ("get_tree", "Tree", "The folders from the root down, at most the storage's depth limit; deeper folders are read with list_children.", schema::get_tree),
    ("list_children", "List children", "The folders and files directly inside one folder; an empty relParts lists the root.", schema::list_children),
    ("get_node", "Node", "One folder or file with its type, status, owner, flags, info.json fields and allowed actions.", schema::get_node),
    ("search_folders", "Search folders", "Folders whose names or fields match the query, from the search index or by walking the disk.", schema::search_folders),
    ("search_files", "Search files", "Files whose names hold every word of the query, inside the folders the filters allow.", schema::search_files),
    ("suggest_search", "Suggest search", "Completions for a `key:` filter while a query is typed: folders for `in`, known values for any other key.", schema::suggest_search),
];

pub(super) fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|(name, title, description, schemas)| {
            let (input, output) = schemas();
            json!({
                "name": name, "title": title, "description": description, "inputSchema": input, "outputSchema": output,
                "annotations": { "title": title, "readOnlyHint": true, "openWorldHint": false },
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TreeArgs {
    depth: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PathArgs {
    rel_parts: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    disk: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuggestArgs {
    key: String,
    prefix: String,
}

fn args<T: DeserializeOwned>(name: &str, arguments: &Value) -> Result<T, RpcError> {
    serde_json::from_value(arguments.clone()).map_err(|e| (INVALID_PARAMS, format!("Invalid arguments for {name}: {e}")))
}

pub(super) fn call(server: &Server, params: &Value) -> Result<Value, RpcError> {
    let name = params.get("name").and_then(Value::as_str).ok_or((INVALID_PARAMS, "tools/call needs a tool name".to_string()))?;
    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let outcome = match name {
        "get_storage" => args::<NoArgs>(name, &arguments).map(|_| server.read(|_, root| StorageInfo::of(root)))?,
        "get_tree" => args::<TreeArgs>(name, &arguments).map(|a| server.read(|_, root| tree(root, a.depth)))?,
        "list_children" => args::<PathArgs>(name, &arguments).map(|a| server.read(|_, root| children(root, &a.rel_parts)))?,
        "get_node" => args::<PathArgs>(name, &arguments).map(|a| server.read(|_, root| node(root, a.rel_parts)))?,
        "search_folders" => args::<SearchArgs>(name, &arguments).map(|a| {
            server.read(|storage, root| {
                let query = parse_query(&a.query);
                let paths = if a.disk { disk_search::search_disk(root, &query)? } else { storage.with_reader(|store| store.search(&query))? };
                Ok(json!({ "paths": paths }))
            })
        })?,
        "search_files" => args::<SearchArgs>(name, &arguments).map(|a| {
            server.read(|_, root| Ok(json!({ "hits": disk_search::search_files(root, &parse_query(&a.query))? })))
        })?,
        "suggest_search" => args::<SuggestArgs>(name, &arguments).map(|a| {
            server.read(|storage, _| Ok(json!({ "suggestions": storage.with_reader(|store| store.suggest(&suggest_key(&a.key), &a.prefix))? })))
        })?,
        _ => return Err((INVALID_PARAMS, format!("Unknown tool: {name}"))),
    };
    Ok(result(outcome))
}

// A refusal is a tool result the agent can read and act on, not a protocol failure.
fn result(outcome: R<Value>) -> Value {
    match outcome {
        Ok(value) => json!({ "content": [{ "type": "text", "text": value.to_string() }], "structuredContent": value, "isError": false }),
        Err(err) => {
            eprintln!("MCP: tool failed: {err}");
            let text = serde_json::to_string(&err).unwrap_or_else(|e| json!({ "kind": "internal", "message": format!("{err} ({e})") }).to_string());
            json!({ "content": [{ "type": "text", "text": text }], "isError": true })
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Tree {
    root: String,
    depth: usize,
    max_depth: usize,
    nodes: Vec<Brief>,
}

// Read as stored, without the sync and index rebuild of a load, so a read never writes to the storage.
fn tree(root: &Path, depth: Option<usize>) -> R<Tree> {
    let max_depth = depth::limit(root)?;
    let depth = depth.unwrap_or(max_depth).clamp(1, max_depth);
    let data = tree_store::load(root)?;
    let mut tree = build_tree(root, &data, false)?;
    fill_children(root, &data, &mut tree.nodes)?;
    Ok(Tree { root: tree.root, depth, max_depth, nodes: tree.nodes.iter().map(|n| Brief::of(n, depth)).collect() })
}

fn children(root: &Path, rel_parts: &[String]) -> R<Value> {
    let (_, kids) = level(root, rel_parts)?;
    Ok(json!({ "nodes": kids.iter().map(|k| Brief::of(k, 1)).collect::<Vec<_>>() }))
}

fn node(root: &Path, mut rel_parts: Vec<String>) -> R<Detail> {
    let Some(name) = rel_parts.pop() else {
        return Err(FmError::InvalidPath { rel_parts, reason: "a node needs at least one folder name".to_string() });
    };
    let (_, kids) = level(root, &rel_parts)?;
    rel_parts.push(name);
    match kids.into_iter().find(|k| k.rel_parts == rel_parts) {
        Some(found) => Ok(Detail::of(found)),
        None => Err(FmError::NotFound { rel_parts }),
    }
}
