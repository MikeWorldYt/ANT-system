// The library of validated extensions published in the repository.
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use super::fail;
use super::fetch::{allowed_url, download, valid_sha256, MAX_ZIP_BYTES, REGISTRY_HOSTS};
use super::manifest::check_info;
use super::store;
use crate::error::R;

const REGISTRY_URL: &str = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Applications/FolderManager/registry.json";
const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct Registry {
    version: u64,
    #[serde(default)]
    extensions: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryItem {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub url: String,
    pub sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    permissions: Vec<String>,
    installed: bool,
}

fn valid_item(i: &RegistryItem) -> bool {
    check_info(&i.id, &i.name, &i.version, &i.description, &i.author, &i.permissions).is_ok()
        && allowed_url(&i.url, &REGISTRY_HOSTS)
        && valid_sha256(&i.sha256)
}

pub fn parse(text: &str) -> R<Vec<RegistryItem>> {
    let registry: Registry =
        serde_yaml::from_str(text).map_err(|_| fail("The extension library is unreadable".to_string()))?;
    if registry.version != 1 {
        return Err(fail(format!("The extension library has an unknown version {}", registry.version)));
    }
    Ok(registry
        .extensions
        .into_iter()
        .filter_map(|v| serde_yaml::from_value::<RegistryItem>(v).ok())
        .filter(valid_item)
        .collect())
}

pub fn fetch() -> R<Vec<RegistryItem>> {
    let bytes = download(REGISTRY_URL, MAX_REGISTRY_BYTES, 20, &REGISTRY_HOSTS, &[])?;
    let text = String::from_utf8(bytes).map_err(|_| fail("The extension library is not UTF-8".to_string()))?;
    parse(&text)
}

pub fn archive(item: &RegistryItem) -> R<Vec<u8>> {
    download(&item.url, MAX_ZIP_BYTES, 60, &REGISTRY_HOSTS, &[])
}

pub fn entries(root: &Path, items: Vec<RegistryItem>) -> Vec<RegistryEntry> {
    items
        .into_iter()
        .map(|i| RegistryEntry {
            installed: store::exists(root, &i.id),
            id: i.id,
            name: i.name,
            version: i.version,
            description: i.description,
            author: i.author,
            permissions: i.permissions,
        })
        .collect()
}
