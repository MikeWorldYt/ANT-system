// Single error type of the core; the frontend switches on the `kind` tag.
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FmError {
    StorageNotFound { start: String },
    YamlParse { message: String, line: Option<usize> },
    InvalidName { name: String, reason: String },
    InvalidPath { rel_parts: Vec<String>, reason: String },
    NotFound { rel_parts: Vec<String> },
    Stale { rel_parts: Vec<String> },
    AlreadyExists { name: String },
    Locked { rel_parts: Vec<String>, node_type: String },
    Reserved { rel_parts: Vec<String> },
    GhostNotAllowed { op: String },
    DepthLimit,
    NotMovable,
    NeedsConfirmation { files: usize, dirs: usize },
    Io { op: String, path: String, message: String, os_code: Option<i32> },
    RolledBack { op: String, cause: String },
    Inconsistent { op: String, cause: String, rollback_errors: Vec<String> },
    Internal { message: String },
}

pub type R<T> = Result<T, FmError>;
