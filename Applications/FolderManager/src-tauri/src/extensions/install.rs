// Turns a checked staging folder into an installed, disabled extension.
use std::fs;
use std::path::Path;

use serde::Serialize;

use super::fail;
use super::fetch::{parse_repo_url, repo_url, same_hash};
use super::manifest;
use super::registry::RegistryItem;
use super::staging::{self, ARCHIVE_FILE};
use super::store::{self, ExtensionInfo, SOURCE_FILE};
use super::tools::sha256_file;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, write_atomic};

const URL_FILE: &str = "source.url";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPreview {
    token: String,
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    permissions: Vec<String>,
    size: u64,
    already_installed: bool,
}

// The url passed allowed_url or parse_repo_url, so it holds no quote or backslash to escape.
fn source_json(source: &str, url: &str, sha256: &str, trust: &str) -> String {
    format!("{{ \"source\": \"{source}\", \"url\": \"{url}\", \"sha256\": \"{sha256}\", \"trust\": \"{trust}\" }}\n")
}

fn load(dir: &Path) -> R<manifest::Manifest> {
    manifest::load(dir).map_err(|reason| fail(format!("The extension is rejected: {reason}")))
}

// The source file is written before the move so a failure never leaves a half-installed folder.
fn place(root: &Path, dir: &Path, id: &str, source: &str) -> R<ExtensionInfo> {
    let target = store::extensions_dir(root).join(id);
    if store::exists(root, id) {
        return Err(FmError::AlreadyExists { name: id.to_string() });
    }
    write_atomic(&dir.join(SOURCE_FILE), source.as_bytes())?;
    fs::rename(dir, &target).map_err(|e| io_err("move folder", &target, &e))?;
    store::forget(root, id)?;
    Ok(store::info(&target, id, false))
}

fn with_staging<T>(dir: &Path, f: impl FnOnce() -> R<T>) -> R<T> {
    let result = f();
    staging::discard(dir);
    result
}

pub fn from_registry(root: &Path, item: &RegistryItem, bytes: &[u8]) -> R<ExtensionInfo> {
    let (_, dir) = staging::create(root, bytes)?;
    with_staging(&dir, || {
        let hash = sha256_file(&dir.join(ARCHIVE_FILE))?;
        if !same_hash(&hash, &item.sha256) {
            return Err(fail(format!("The download of {} does not match the library checksum", item.id)));
        }
        let content = staging::manifest_dir(&dir)?;
        let manifest = load(&content)?;
        if manifest.id != item.id {
            return Err(fail(format!("The archive holds {} instead of {}", manifest.id, item.id)));
        }
        place(root, &content, &item.id, &source_json("registry", &item.url, &hash, "validated"))
    })
}

pub fn stage_repo(root: &Path, url: &str, bytes: &[u8]) -> R<ExtensionPreview> {
    staging::clear_old(root)?;
    let (token, dir) = staging::create(root, bytes)?;
    let preview = (|| {
        let url_file = dir.join(URL_FILE);
        fs::write(&url_file, url).map_err(|e| io_err("write file", &url_file, &e))?;
        let m = load(&staging::manifest_dir(&dir)?)?;
        Ok(ExtensionPreview {
            already_installed: store::exists(root, &m.id),
            token,
            id: m.id,
            name: m.name,
            version: m.version,
            description: m.description,
            author: m.author,
            permissions: m.permissions,
            size: bytes.len() as u64,
        })
    })();
    if preview.is_err() {
        staging::discard(&dir);
    }
    preview
}

pub fn from_staging(root: &Path, token: &str) -> R<ExtensionInfo> {
    let dir = staging::open(root, token)?;
    with_staging(&dir, || {
        let url_file = dir.join(URL_FILE);
        let saved = manifest::read_small(&url_file, 4096).map_err(fail)?;
        let (owner, repo) = std::str::from_utf8(&saved)
            .ok()
            .and_then(parse_repo_url)
            .ok_or_else(|| fail("The preview lost its repository address".to_string()))?;
        let hash = sha256_file(&dir.join(ARCHIVE_FILE))?;
        let content = staging::manifest_dir(&dir)?;
        let manifest = load(&content)?;
        place(root, &content, &manifest.id, &source_json("repository", &repo_url(&owner, &repo), &hash, "unvalidated"))
    })
}
