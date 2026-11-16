// The write gate through tools/call: read-only by default, a yes from the window for destructive calls and batches, and per-call limits.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::gate::{Gate, WRITES_KEY};
use super::tests::{call, exchange, failed, ok, request, server};
use super::tools::DATA_NOTE;
use super::Server;
use crate::infra::storage::db::Store;
use crate::services::bridge::endpoint::{self, Endpoint};
use crate::services::bridge::server::{bind, serve};
use crate::services::bridge::{Reply, UiCall};
use crate::test_support::{ok_recycle, work, TempDir};

type Calls = Arc<Mutex<Vec<UiCall>>>;

fn allow_writes(root: &Path) {
    Store::open_or_create(root).unwrap().set_setting(WRITES_KEY, "true").unwrap();
}

// A stand-in window that answers every confirmation the same way, after `delay`.
fn window(open: PathBuf, dir: &Path, delay: Duration, reply: fn() -> Reply) -> (PathBuf, Calls) {
    let (listener, port) = bind().unwrap();
    let token = endpoint::new_token().unwrap();
    let calls = Calls::default();
    let (seen, secret) = (calls.clone(), token.clone());
    std::thread::spawn(move || {
        serve(listener, secret, move || Some(open.clone()), move |c| {
            seen.lock().unwrap().push(c);
            std::thread::sleep(delay);
            reply()
        })
    });
    let file = endpoint::file(dir, Path::new("C:\\Apps\\fm.exe"));
    endpoint::write(&file, &Endpoint { port, token }).unwrap();
    (file, calls)
}

fn gated(root: &Path, bridge: Option<PathBuf>) -> Server {
    let mut s = server(root);
    s.recycle = ok_recycle;
    if let Some(file) = bridge {
        s.bridge = Ok(file);
    }
    s
}

fn yes() -> Reply {
    Ok(json!({ "confirmed": true }))
}

fn no() -> Reply {
    Ok(json!({ "confirmed": false }))
}

#[test]
fn writes_are_refused_until_the_user_turns_them_on_and_reads_still_work() {
    let dir = work("mcp-g-readonly");
    let bridges = TempDir::new("mcp-g-readonly-bridge");
    let (file, calls) = window(dir.path().to_path_buf(), bridges.path(), Duration::ZERO, yes);
    let s = gated(dir.path(), Some(file));
    for (tool, arguments) in [
        ("rename", json!({ "relParts": ["100.WORK", "02.B"], "name": "Bee" })),
        ("add_node", json!({ "parent": ["100.WORK"], "name": "new" })),
        ("delete", json!({ "relParts": ["100.WORK", "02.B"], "confirmNonEmpty": true })),
        ("set_dictionary", json!({ "relParts": ["100.WORK"], "entries": [] })),
    ] {
        let refused = failed(&call(&s, tool, arguments));
        let message = refused["message"].as_str().unwrap();
        assert!(message.contains("only read") && message.contains("Settings > Advanced") && message.contains("nothing was changed"), "{tool}: {refused}");
    }
    assert!(dir.path().join("100.WORK/02.B").is_dir() && !dir.path().join("100.WORK/.ant/dictionary.md").exists());
    assert!(calls.lock().unwrap().is_empty());
    ok(&call(&s, "get_tree", json!({})));
    allow_writes(dir.path());
    ok(&call(&s, "rename", json!({ "relParts": ["100.WORK", "02.B"], "name": "Bee" })));
    assert!(dir.path().join("100.WORK/02.Bee").is_dir());
    assert!(calls.lock().unwrap().is_empty(), "a single rename asks no one");
}

#[test]
fn a_destructive_call_runs_only_after_the_window_says_yes() {
    let dir = work("mcp-g-yes");
    allow_writes(dir.path());
    let bridges = TempDir::new("mcp-g-yes-bridge");
    let (file, calls) = window(dir.path().to_path_buf(), bridges.path(), Duration::ZERO, yes);
    ok(&call(&gated(dir.path(), Some(file)), "delete", json!({ "relParts": ["100.WORK", "02.B"] })));
    assert!(!dir.path().join("100.WORK/02.B").exists());
    assert_eq!(*calls.lock().unwrap(), [UiCall::Confirm { title: "Delete folder".to_string(), items: vec!["100.WORK/02.B".to_string()], seconds: 60 }]);
}

#[test]
fn a_declined_confirmation_changes_nothing() {
    let dir = work("mcp-g-no");
    allow_writes(dir.path());
    let bridges = TempDir::new("mcp-g-no-bridge");
    let (file, calls) = window(dir.path().to_path_buf(), bridges.path(), Duration::ZERO, no);
    let s = gated(dir.path(), Some(file));
    let declined = failed(&call(&s, "delete", json!({ "relParts": ["100.WORK", "02.B"] })));
    assert!(declined["message"].as_str().unwrap().contains("declined"), "{declined}");
    let moved = failed(&call(&s, "move", json!({ "source": ["100.WORK", "03.C"], "destination": ["100.WORK", "01.A"] })));
    assert!(moved["message"].as_str().unwrap().contains("declined"), "{moved}");
    failed(&call(&s, "set_dictionary", json!({ "relParts": ["100.WORK"], "entries": [] })));
    let cards = json!({ "cards": [{ "relParts": ["100.WORK", "04.Wkb"], "column": "todo", "order": 0 }, { "relParts": ["100.WORK", "02.B"], "column": "todo", "order": 1 }] });
    failed(&call(&s, "set_board", cards));
    assert!(dir.path().join("100.WORK/02.B").is_dir() && dir.path().join("100.WORK/03.C").is_dir() && !dir.path().join("100.WORK/.ant/dictionary.md").exists());
    let asked: Vec<String> = calls.lock().unwrap().iter().map(|c| match c { UiCall::Confirm { title, .. } => title.clone(), other => format!("{other:?}") }).collect();
    assert_eq!(asked, ["Delete folder", "Move folder", "Write dictionary", "Place board cards"]);
}

#[test]
fn an_unanswered_confirmation_times_out_and_changes_nothing() {
    let dir = work("mcp-g-timeout");
    allow_writes(dir.path());
    let bridges = TempDir::new("mcp-g-timeout-bridge");
    let (file, _) = window(dir.path().to_path_buf(), bridges.path(), Duration::from_secs(6), yes);
    let mut s = gated(dir.path(), Some(file));
    s.confirm_wait = Duration::from_secs(1);
    let started = Instant::now();
    let late = failed(&call(&s, "delete", json!({ "relParts": ["100.WORK", "02.B"] })));
    assert!(late["message"].as_str().unwrap().contains("did not confirm"), "{late}");
    assert!(started.elapsed() < Duration::from_secs(6), "{:?}", started.elapsed());
    assert!(dir.path().join("100.WORK/02.B").is_dir());
}

#[test]
fn without_the_app_a_destructive_call_is_refused() {
    let dir = work("mcp-g-closed");
    allow_writes(dir.path());
    let bridges = TempDir::new("mcp-g-closed-bridge");
    let s = gated(dir.path(), Some(endpoint::file(bridges.path(), Path::new("C:\\Apps\\fm.exe"))));
    let refused = failed(&call(&s, "delete_file", json!({ "relParts": ["100.WORK", "02.B", "a.txt"] })));
    assert!(refused["message"].as_str().unwrap().contains("not running"), "{refused}");
    ok(&call(&s, "set_favorite", json!({ "relParts": ["100.WORK", "02.B"], "on": true })));
}

#[test]
fn a_batch_over_the_limit_is_refused_before_anything_runs() {
    let dir = work("mcp-g-limit");
    let mut s = server(dir.path());
    s.gate = Gate::Open;
    let cards: Vec<Value> = (0..51).map(|i| json!({ "relParts": ["100.WORK", "04.Wkb"], "column": "todo", "order": i })).collect();
    let plans: Vec<Value> = (0..51).map(|i| json!({ "name": format!("PJ_{i}"), "files": [] })).collect();
    let files: Vec<String> = (0..501).map(|i| format!("f{i}.txt")).collect();
    let input = request(1, "tools/call", json!({ "name": "set_board", "arguments": { "cards": cards } }))
        + &request(2, "tools/call", json!({ "name": "apply_project_groups", "arguments": { "folder": ["100.WORK", "98.Rev"], "plans": plans } }))
        + &request(3, "tools/call", json!({ "name": "apply_project_groups", "arguments": { "folder": ["100.WORK", "98.Rev"], "plans": [{ "name": "PJ_X", "files": files }] } }));
    let replies = exchange(&s, input.as_bytes());
    assert_eq!(replies.len(), 3);
    for (reply, what) in replies.iter().zip(["51 cards", "51 plans", "501 files"]) {
        assert_eq!(reply["error"]["code"], -32602, "{reply}");
        assert!(reply["error"]["message"].as_str().unwrap().contains(what), "{reply}");
    }
    let board = ok(&call(&s, "get_node", json!({ "relParts": ["100.WORK", "04.Wkb"] })));
    assert!(!board.to_string().contains("todo"), "{board}");
}

#[test]
fn results_mark_names_as_data_and_quote_them_in_the_change_note() {
    let dir = work("mcp-g-data");
    let mut s = server(dir.path());
    s.gate = Gate::Open;
    let result = call(&s, "add_node", json!({ "parent": ["100.WORK"], "name": "fresh notes" }));
    assert_eq!(result["content"][1]["text"], DATA_NOTE);
    assert_eq!(ok(&result)["changed"], r#"Added a folder "fresh notes" in "/100.WORK""#);
    let renamed = ok(&call(&s, "rename", json!({ "relParts": ["100.WORK", "02.B"], "name": "Bee" })));
    assert_eq!(renamed["changed"], r#"Renamed "100.WORK/02.B""#);
    assert_eq!(failed(&call(&s, "get_node", json!({ "relParts": ["100.WORK", "99.Missing"] })))["kind"], "notFound");
    assert_eq!(call(&s, "get_node", json!({ "relParts": ["100.WORK", "99.Missing"] }))["content"][1]["text"], DATA_NOTE);
}
