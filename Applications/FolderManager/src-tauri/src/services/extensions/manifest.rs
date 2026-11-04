// Rules an extension's manifest must pass before the app lists it as usable.
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::fs_safe::{is_link, validate_component};

pub const PERMISSIONS: [&str; 4] = ["tree:read", "menu:context", "panel:own", "details:field"];
pub const MAX_MAIN_BYTES: u64 = 1024 * 1024;
pub const MAX_JSON_BYTES: u64 = 64 * 1024;
const MANIFEST_FILE: &str = "manifest.json";
const MAX_MAIN_CHARS: usize = 200;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub author: String,
    pub api_version: u64,
    pub main: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

// The device-name check matters because the id is also a folder name on Windows.
pub fn valid_id(id: &str) -> bool {
    (3..=40).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && validate_component(id).is_ok()
}

fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| (1..=9).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

fn text(field: &str, value: &str, min: usize, max: usize) -> Result<(), String> {
    let n = value.chars().count();
    if n < min || n > max || value.chars().any(char::is_control) {
        return Err(format!("{field} must be {min}-{max} characters without control characters"));
    }
    Ok(())
}

// Shared by manifests and registry entries.
pub fn check_info(
    id: &str,
    name: &str,
    version: &str,
    description: &str,
    author: &str,
    permissions: &[String],
) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id must be 3-40 lowercase letters, digits or -".to_string());
    }
    text("name", name, 1, 60)?;
    if !valid_version(version) {
        return Err("version must be major.minor.patch".to_string());
    }
    text("description", description, 0, 200)?;
    text("author", author, 1, 60)?;
    if permissions.is_empty() {
        return Err("declares no permissions".to_string());
    }
    if let Some(unknown) = permissions.iter().find(|p| !PERMISSIONS.contains(&p.as_str())) {
        return Err(format!("unknown permission {unknown:?}"));
    }
    Ok(())
}

pub fn main_parts(main: &str) -> Option<Vec<&str>> {
    if main.chars().count() > MAX_MAIN_CHARS || !main.ends_with(".js") || main.contains('\\') || main.starts_with('/') {
        return None;
    }
    let parts: Vec<&str> = main.split('/').collect();
    parts.iter().all(|p| validate_component(p).is_ok()).then_some(parts)
}

pub fn parse(text: &str) -> Result<Manifest, String> {
    let manifest: Manifest = serde_yaml::from_str(text).map_err(|e| {
        format!("manifest.json is invalid: {}", e.to_string().lines().next().unwrap_or_default())
    })?;
    let m = &manifest;
    check_info(&m.id, &m.name, &m.version, &m.description, &m.author, &m.permissions)?;
    if m.api_version != 1 {
        return Err(format!("apiVersion {} is not supported", m.api_version));
    }
    if main_parts(&m.main).is_none() {
        return Err("main must be a relative .js file inside the folder".to_string());
    }
    Ok(manifest)
}

pub fn read_small(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(format!("{name} is missing")),
        Err(e) => return Err(format!("{name} cannot be read: {e}")),
    };
    if is_link(&meta) || !meta.is_file() {
        return Err(format!("{name} is not a plain file"));
    }
    if meta.len() > max {
        return Err(format!("{name} is larger than {} KB", max / 1024));
    }
    fs::read(path).map_err(|e| format!("{name} cannot be read: {e}"))
}

// Every step is checked so a link inside the folder cannot point the entry file elsewhere.
pub fn main_file(dir: &Path, main: &str) -> Result<PathBuf, String> {
    let parts = main_parts(main).ok_or_else(|| "main must be a relative .js file inside the folder".to_string())?;
    let mut path = dir.to_path_buf();
    for part in parts {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) if is_link(&meta) => return Err(format!("{main} goes through a link")),
            Ok(_) => {}
            Err(_) => return Err(format!("{main} is missing")),
        }
    }
    read_small_meta(&path, main)?;
    Ok(path)
}

fn read_small_meta(path: &Path, main: &str) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{main} cannot be read: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{main} is not a file"));
    }
    if meta.len() > MAX_MAIN_BYTES {
        return Err(format!("{main} is larger than 1 MB"));
    }
    Ok(())
}

pub fn load(dir: &Path) -> Result<Manifest, String> {
    let bytes = read_small(&dir.join(MANIFEST_FILE), MAX_JSON_BYTES)?;
    let text = String::from_utf8(bytes).map_err(|_| "manifest.json is not UTF-8".to_string())?;
    let manifest = parse(&text)?;
    main_file(dir, &manifest.main)?;
    Ok(manifest)
}

pub fn has_manifest(dir: &Path) -> bool {
    fs::symlink_metadata(dir.join(MANIFEST_FILE)).is_ok_and(|m| m.is_file() && !is_link(&m))
}
