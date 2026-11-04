// Guards against a command registered in Rust but never called from the frontend API, or the reverse.
use std::collections::BTreeSet;

const LIB_RS: &str = include_str!("lib.rs");
const API_TS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/ipc/api.ts"));

fn handler_names(src: &str) -> BTreeSet<String> {
    let start = src.find("generate_handler![").expect("generate_handler! not found") + "generate_handler![".len();
    let end = start + src[start..].find(']').expect("generate_handler! is not closed");
    src[start..end]
        .split(',')
        .map(|entry| entry.trim().rsplit("::").next().unwrap_or_default().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

// Reads `call<...>("name"`; the `call<T>(cmd` definition has no string literal and is skipped.
fn api_names(src: &str) -> BTreeSet<String> {
    let bytes = src.as_bytes();
    let mut names = BTreeSet::new();
    for (at, _) in src.match_indices("call") {
        if at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_') {
            continue;
        }
        let mut i = at + "call".len();
        if bytes.get(i) == Some(&b'<') {
            let mut depth = 0;
            while let Some(&b) = bytes.get(i) {
                match b {
                    b'<' => depth += 1,
                    b'>' => depth -= 1,
                    _ => {}
                }
                i += 1;
                if depth == 0 {
                    break;
                }
            }
        }
        if bytes.get(i) != Some(&b'(') {
            continue;
        }
        let rest = src[i + 1..].trim_start();
        let Some(literal) = rest.strip_prefix('"') else { continue };
        if let Some(close) = literal.find('"') {
            names.insert(literal[..close].to_string());
        }
    }
    names
}

#[test]
fn handler_commands_match_frontend_api() {
    let rust = handler_names(LIB_RS);
    let api = api_names(API_TS);
    assert!(!rust.is_empty() && !api.is_empty(), "no command names extracted");
    let only_rust: Vec<_> = rust.difference(&api).collect();
    let only_api: Vec<_> = api.difference(&rust).collect();
    assert!(
        only_rust.is_empty() && only_api.is_empty(),
        "only in generate_handler!: {only_rust:?}; only in api.ts: {only_api:?}"
    );
}
