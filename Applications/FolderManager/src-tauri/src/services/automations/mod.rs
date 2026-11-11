// Automations that tidy the files of review folders.
mod batch;
mod dict_match;
pub mod dictionary;
mod plan;
pub mod reduce_name;

pub use batch::reduce_files;
pub use plan::{apply, plan, PlannedFile};
