// Automations that tidy the files of review folders.
mod batch;
mod dict_match;
pub mod dictionary;
pub mod dictionary_lint;
mod plan;
pub mod reduce_name;
#[cfg(test)]
mod dict_tests;
#[cfg(test)]
mod plan_tests;

pub use batch::reduce_files;
pub use plan::{apply, plan, Picks, PlannedFile};
