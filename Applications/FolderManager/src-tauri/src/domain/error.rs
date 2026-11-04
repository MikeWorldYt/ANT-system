// Single error type of the core; the frontend switches on the `kind` tag.
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FmError {
    StorageNotFound { start: String },
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
    ClipboardEmpty,
}

// Readable text for causes nested inside RolledBack, Inconsistent and warnings.
impl std::fmt::Display for FmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageNotFound { start } => write!(f, "No ANT storage found from {start}"),
            Self::InvalidName { name, reason } => write!(f, "Invalid name \"{name}\": {reason}"),
            Self::InvalidPath { rel_parts, reason } => {
                write!(f, "Invalid path \"{}\": {reason}", rel_parts.join("/"))
            }
            Self::NotFound { rel_parts } => write!(f, "{} was not found", rel_parts.join("/")),
            Self::Stale { rel_parts } => write!(f, "{} changed on disk; refresh", rel_parts.join("/")),
            Self::AlreadyExists { name } => write!(f, "{name} already exists"),
            Self::Locked { rel_parts, node_type } => {
                write!(f, "{} is a locked {node_type} folder", rel_parts.join("/"))
            }
            Self::Reserved { rel_parts } => write!(f, "{} is reserved", rel_parts.join("/")),
            Self::GhostNotAllowed { op } => write!(f, "Cannot {op} a folder that is missing on disk"),
            Self::DepthLimit => write!(f, "The maximum depth is reached"),
            Self::NotMovable => write!(f, "The folder cannot move that way"),
            Self::NeedsConfirmation { files, dirs } => {
                write!(f, "The folder holds {files} files and {dirs} folders")
            }
            Self::Io { op, path, message, .. } => write!(f, "{op} {path}: {message}"),
            Self::RolledBack { op, cause } => write!(f, "{op} was undone: {cause}"),
            Self::Inconsistent { op, cause, rollback_errors } => {
                write!(f, "{op} failed ({cause}) and was not fully undone: {}", rollback_errors.join("; "))
            }
            Self::Internal { message } => write!(f, "{message}"),
            Self::ClipboardEmpty => write!(f, "The clipboard has no files to paste"),
        }
    }
}

pub type R<T> = Result<T, FmError>;
