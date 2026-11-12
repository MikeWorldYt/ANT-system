// The ui_* tools through the protocol, against a stand-in for the running app on a real loopback bridge.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::tests::{call, exchange, failed, ok, request, server};
use super::Server;
use crate::services::bridge::endpoint::{self, Endpoint};
use crate::services::bridge::server::{bind, serve};
use crate::services::bridge::UiCall;
use crate::test_support::{rel, work, TempDir};

type Calls = Arc<Mutex<Vec<UiCall>>>;

// The stand-in window answers with a state that echoes the call, and keeps what it was asked.
fn app_on(open: PathBuf, dir: &Path) -> (PathBuf, Calls) {
    let (listener, port) = bind().unwrap();
    let token = endpoint::new_token().unwrap();
    let calls = Calls::default();
    let seen = calls.clone();
    let secret = token.clone();
    std::thread::spawn(move || {
        serve(listener, secret, move || Some(open.clone()), move |call| {
            seen.lock().unwrap().push(call.clone());
            Ok(json!({ "view": "tree", "call": call }))
        })
    });
    let file = endpoint::file(dir, Path::new("C:\\Apps\\fm.exe"));
    endpoint::write(&file, &Endpoint { port, token }).unwrap();
    (file, calls)
}

fn bridged(root: &Path, file: PathBuf) -> Server {
    let mut s = server(root);
    s.bridge = Ok(file);
    s
}

#[test]
fn the_ui_tools_are_listed_and_only_ui_get_state_is_read_only() {
    let dir = work("mcp-ui-list");
    let replies = exchange(&server(dir.path()), request(1, "tools/list", json!({})).as_bytes());
    let tools: Vec<&Value> = replies[0]["result"]["tools"].as_array().unwrap().iter().filter(|t| t["name"].as_str().unwrap().starts_with("ui_")).collect();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["ui_get_state", "ui_reveal", "ui_open_tab", "ui_split", "ui_set_view"]);
    assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
    for tool in &tools[1..] {
        assert_eq!((tool["annotations"]["readOnlyHint"].clone(), tool["annotations"]["destructiveHint"].clone()), (json!(false), json!(false)), "{tool}");
    }
    assert_eq!(tools[2]["annotations"]["idempotentHint"], false);
    assert_eq!(tools[4]["inputSchema"]["properties"]["view"]["enum"], json!(["cards", "tree", "columns", "grid", "list"]));
}

#[test]
fn each_ui_tool_reaches_the_running_app_with_its_call() {
    let dir = work("mcp-ui-calls");
    let bridges = TempDir::new("mcp-ui-bridge");
    let (file, calls) = app_on(dir.path().to_path_buf(), bridges.path());
    let s = bridged(dir.path(), file);
    let state = ok(&call(&s, "ui_get_state", json!({})));
    assert_eq!(state, json!({ "view": "tree", "call": { "action": "getState" } }));
    ok(&call(&s, "ui_reveal", json!({ "relParts": ["100.WORK", "01.A"] })));
    ok(&call(&s, "ui_open_tab", json!({})));
    ok(&call(&s, "ui_open_tab", json!({ "relParts": [] })));
    ok(&call(&s, "ui_split", json!({ "relParts": ["100.WORK"] })));
    ok(&call(&s, "ui_set_view", json!({ "view": "columns" })));
    let expected = [
        UiCall::GetState,
        UiCall::Reveal { rel_parts: rel(&["100.WORK", "01.A"]) },
        UiCall::OpenTab { rel_parts: None },
        UiCall::OpenTab { rel_parts: None },
        UiCall::Split { rel_parts: Some(rel(&["100.WORK"])) },
        UiCall::SetView { view: "columns".to_string() },
    ];
    assert_eq!(*calls.lock().unwrap(), expected);
}

#[test]
fn bad_paths_and_views_are_refused_before_the_app_is_asked() {
    let dir = work("mcp-ui-refused");
    std::fs::write(dir.path().join("100.WORK/01.A/notes.txt"), "x").unwrap();
    let bridges = TempDir::new("mcp-ui-refused-bridge");
    let (file, calls) = app_on(dir.path().to_path_buf(), bridges.path());
    let s = bridged(dir.path(), file);
    assert_eq!(failed(&call(&s, "ui_reveal", json!({ "relParts": ["100.WORK", "99.Missing"] })))["kind"], "notFound");
    assert_eq!(failed(&call(&s, "ui_reveal", json!({ "relParts": [] })))["kind"], "invalidPath");
    assert!(failed(&call(&s, "ui_split", json!({ "relParts": [".."] })))["kind"].is_string());
    assert_eq!(failed(&call(&s, "ui_open_tab", json!({ "relParts": ["100.WORK", "01.A", "notes.txt"] })))["kind"], "invalidPath");
    let input = request(1, "tools/call", json!({ "name": "ui_set_view", "arguments": { "view": "board" } }))
        + &request(2, "tools/call", json!({ "name": "ui_reveal", "arguments": {} }))
        + &request(3, "tools/call", json!({ "name": "ui_get_state", "arguments": { "token": "x" } }))
        + &request(4, "tools/call", json!({ "name": "ui_move_mouse", "arguments": {} }));
    let codes: Vec<Value> = exchange(&s, input.as_bytes()).iter().map(|r| r["error"]["code"].clone()).collect();
    assert_eq!(codes, [json!(-32602), json!(-32602), json!(-32602), json!(-32602)]);
    ok(&call(&s, "ui_reveal", json!({ "relParts": ["100.WORK", "01.A", "notes.txt"] })));
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn without_the_app_ui_tools_say_so_and_background_tools_still_work() {
    let dir = work("mcp-ui-closed");
    let bridges = TempDir::new("mcp-ui-closed-bridge");
    let s = bridged(dir.path(), endpoint::file(bridges.path(), Path::new("C:\\Apps\\fm.exe")));
    for (tool, arguments) in [("ui_get_state", json!({})), ("ui_reveal", json!({ "relParts": ["100.WORK"] })), ("ui_set_view", json!({ "view": "tree" }))] {
        let err = failed(&call(&s, tool, arguments));
        assert!(err["message"].as_str().unwrap().contains("not running"), "{tool}: {err}");
    }
    ok(&call(&s, "get_tree", json!({})));
}

#[test]
fn an_app_on_another_storage_refuses_to_move_its_window() {
    let dir = work("mcp-ui-other");
    let other = work("mcp-ui-other-open");
    let bridges = TempDir::new("mcp-ui-other-bridge");
    let (file, calls) = app_on(other.path().to_path_buf(), bridges.path());
    let err = failed(&call(&bridged(dir.path(), file), "ui_get_state", json!({})));
    assert!(err["message"].as_str().unwrap().contains("another storage open"), "{err}");
    assert!(calls.lock().unwrap().is_empty());
}
