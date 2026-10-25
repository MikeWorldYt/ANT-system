// Creates a new ANT Storage from Tree.yaml; shared by the CLI and any future front end.
mod config;
mod create;
mod downloads;
mod fetch;
mod fs_ops;
mod name;
mod plan;
#[cfg(windows)]
mod powershell;
mod shortcuts;
mod tree_types;
#[cfg(test)]
mod test_dir;
#[cfg(test)]
mod tests;

pub use create::{create, CreateOptions, Progress};
pub use name::sanitize_name;
pub use plan::{plan, PlanNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Step,
    Info,
    Ok,
    Warn,
    Error,
}
