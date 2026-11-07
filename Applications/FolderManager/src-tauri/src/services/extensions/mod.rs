// Sandboxed JavaScript extensions: their folders under .ant/extensions, the validated library and installs.
mod confirmed;
mod fetch;
mod install;
mod manifest;
mod registry;
pub mod sandbox_page;
mod staging;
mod store;
#[cfg(test)]
mod confirm_tests;
#[cfg(test)]
mod tests;
mod tools;
mod zip_limits;
#[cfg(test)]
mod zip_tests;

use crate::domain::error::{FmError, R};

pub use fetch::{parse_repo_url, repo_url, repo_zip};
pub use install::{from_registry, from_staging, stage_registry, stage_repo, ExtensionPreview};
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
