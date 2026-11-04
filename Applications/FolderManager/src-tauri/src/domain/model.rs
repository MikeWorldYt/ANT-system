// Plain data types passed between the index, the tree and the UI.
use serde::Serialize;

// Only a refresh the user sees tails copies, because each tailed copy must reach the question exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Copies {
    Ignore,
    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Copied {
    pub rel_parts: Vec<String>,
    pub original: Vec<String>,
}
