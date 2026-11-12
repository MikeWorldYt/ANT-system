// The ANT guide for agents: a short base sent at initialize, and topics an agent reads one at a time as resources.
use serde_json::{json, Value};

pub(super) const BASE: &str = include_str!("guide/guide.md");

const TOPICS: [(&str, &str, &str); 5] = [
    ("model", "Levels, ids and folder types", include_str!("guide/model.md")),
    ("review", "Review folders and their automations", include_str!("guide/review.md")),
    ("dictionary", "The .ant/dictionary.md grouping format", include_str!("guide/dictionary.md")),
    ("workboards", "Workboards, statuses and the board", include_str!("guide/workboards.md")),
    ("safety", "Locks, confirmations and what never to do", include_str!("guide/safety.md")),
];

const ROOT: &str = "ant://guide";

pub(super) fn list() -> Value {
    let topics = TOPICS.iter().map(|(name, title, _)| json!({ "uri": format!("{ROOT}/{name}"), "name": name, "title": title, "mimeType": "text/markdown" }));
    let base = json!({ "uri": ROOT, "name": "guide", "title": "ANT basics and topic index", "mimeType": "text/markdown" });
    json!({ "resources": std::iter::once(base).chain(topics).collect::<Vec<_>>() })
}

pub(super) fn read(uri: &str) -> Option<Value> {
    let text = if uri == ROOT { BASE } else { TOPICS.iter().find(|(name, _, _)| uri.strip_prefix(ROOT).and_then(|r| r.strip_prefix('/')) == Some(*name))?.2 };
    Some(json!({ "contents": [{ "uri": uri, "mimeType": "text/markdown", "text": text }] }))
}
