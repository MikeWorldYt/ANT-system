// Sandboxed JavaScript extensions: their folders under .ant/extensions, the validated library and installs.
mod fetch;
mod install;
mod manifest;
mod registry;
pub mod sandbox_page;
mod staging;
mod store;
#[cfg(test)]
mod tests;
mod tools;

use crate::error::{FmError, R};

pub use fetch::{parse_repo_url, repo_url, repo_zip};
pub use install::{from_registry, from_staging, stage_repo, ExtensionPreview};
pub use registry::{archive, entries, fetch as fetch_registry, RegistryEntry};
pub use store::{list, read_entry, remove, set_enabled, ExtensionInfo};

fn fail(message: String) -> FmError {
    FmError::Internal { message }
}

pub fn check_id(id: &str) -> R<()> {
    if !manifest::valid_id(id) {
        return Err(fail(format!("Invalid extension id {id:?}")));
    }
    Ok(())
}
