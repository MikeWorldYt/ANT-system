// Every write tool driven through tools/call against throwaway storages, the way an agent changes one.
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use serde_json::{json, Value};

use super::retry::Retry;
use super::tests::{call, exchange, failed, names, ok, request, server};
use super::Server;
use crate::infra::fs_safe::{StorageLock, LOCK_FILE};
use crate::infra::storage::changes;
use crate::services::tree::tree_store::{get_node, get_type};
use crate::test_support::{rel, storage, work, yaml};

// Stands in for the Recycle Bin, which starts PowerShell.
fn bin(path: &Path) -> Result<(), String> {
    if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) }.map_err(|e| e.to_string())
}

fn writer(root: &Path) -> Server {
    let mut s = server(root);
    s.recycle = bin;
    s
}

fn in_tree(root: &Path, parts: &[&str]) -> bool {
    get_node(&yaml(root), &rel(parts)).is_some()
}

fn node(s: &Server, parts: &[&str]) -> Value {
    ok(&call(s, "get_node", json!({ "relParts": parts })))
}

fn info(root: &Path, folder: &str) -> String {
    fs::read_to_string(root.join(folder).join(".ant/info.json")).unwrap()
}

#[test]
fn write_tools_declare_what_they_do() {
    let dir = work("mcp-w-list");
    let replies = exchange(&server(dir.path()), request(1, "tools/list", json!({})).as_bytes());
    let tools = replies[0]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 31);
    let hints = |name: &str| {
        let a = &tools.iter().find(|t| t["name"] == name).unwrap()["annotations"];
        (a["readOnlyHint"].clone(), a["destructiveHint"].clone(), a["idempotentHint"].clone())
    };
    for name in ["move", "reorder", "delete", "delete_file", "apply_project_groups", "undo_project_sort"] {
        assert_eq!(hints(name), (json!(false), json!(true), json!(false)), "{name}");
    }
    for name in ["set_type", "set_status", "set_owner", "set_favorite", "set_important", "hide", "set_board"] {
        assert_eq!(hints(name), (json!(false), json!(false), json!(true)), "{name}");
    }
    for name in ["add_node", "add_workboard", "rename", "copy", "rename_file"] {
        assert_eq!(hints(name), (json!(false), json!(false), json!(false)), "{name}");
    }
    assert_eq!(hints("preview_project_groups").0, json!(true));
}

#[test]
fn add_and_rename_change_disk_tree_and_search_and_answer_the_new_path() {
    let dir = work("mcp-w-add");
    let s = writer(dir.path());
    let added = ok(&call(&s, "add_node", json!({ "parent": ["100.WORK"], "name": "fresh notes" })));
    assert_eq!((added["relParts"].clone(), added["warnings"].clone()), (json!(["100.WORK", "06.Fresh-notes"]), json!([])));
    assert!(added["fullTree"] == true || added["levels"].as_array().unwrap().contains(&json!(["100.WORK"])), "{added}");
    assert!(dir.path().join("100.WORK/06.Fresh-notes").is_dir() && in_tree(dir.path(), &["100.WORK", "06.Fresh-notes"]));
    let wkb = ok(&call(&s, "add_workboard", json!({ "parent": ["100.WORK", "02.B"], "name": "sprint" })));
    assert_eq!(wkb["relParts"], json!(["100.WORK", "02.B", "PJ.001_Sprint"]));
    assert!(get_type(&yaml(dir.path()), &rel(&["100.WORK", "02.B", "PJ.001_Sprint"])).starts_with("WKB"));
    let renamed = ok(&call(&s, "rename", json!({ "relParts": ["100.WORK", "03.C"], "name": "see" })));
    assert_eq!(renamed["relParts"], json!(["100.WORK", "03.See"]));
    assert!(dir.path().join("100.WORK/03.See").is_dir() && !dir.path().join("100.WORK/03.C").exists());
    let paths = ok(&call(&s, "search_folders", json!({ "query": "see" })))["paths"].clone();
    assert!(paths.as_array().unwrap().contains(&json!(["100.WORK", "03.See"])), "{paths}");
}

#[test]
fn move_copy_and_reorder_follow_the_app_rules() {
    let dir = work("mcp-w-move");
    let s = writer(dir.path());
    let moved = ok(&call(&s, "move", json!({ "source": ["100.WORK", "03.C"], "destination": ["100.WORK", "01.A"] })));
    let at = moved["relParts"].as_array().unwrap();
    assert_eq!((at.len(), at[1].clone()), (3, json!("01.A")), "{moved}");
    assert!(!dir.path().join("100.WORK/03.C").exists() && dir.path().join("100.WORK/01.A").join(at[2].as_str().unwrap()).is_dir());
    let copied = ok(&call(&s, "copy", json!({ "source": ["100.WORK", "02.B"], "destination": ["100.WORK", "01.A"] })));
    assert!(dir.path().join("100.WORK/02.B").is_dir(), "{copied}");
    assert!(dir.path().join("100.WORK/01.A").join(copied["relParts"][2].as_str().unwrap()).is_dir());
    let inside = failed(&call(&s, "move", json!({ "source": ["100.WORK", "01.A"], "destination": ["100.WORK", "01.A", "01.Keep"] })));
    assert_eq!(inside["kind"], "invalidPath", "{inside}");
    ok(&call(&s, "reorder", json!({ "relParts": ["100.WORK", "01.A"], "direction": "down" })));
    let top = fs::read_dir(dir.path().join("100.WORK")).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect::<Vec<_>>();
    assert!(top.contains(&"02.A".to_string()) && top.contains(&"01.B".to_string()), "{top:?}");
}

#[test]
fn delete_goes_to_the_bin_only_with_the_flags_the_ui_confirms() {
    let dir = work("mcp-w-delete");
    let s = writer(dir.path());
    fs::write(dir.path().join("100.WORK/02.B/a.txt"), "x").unwrap();
    let refused = failed(&call(&s, "delete", json!({ "relParts": ["100.WORK", "02.B"] })));
    assert_eq!(refused, json!({ "kind": "needsConfirmation", "files": 1, "dirs": 0 }));
    assert!(dir.path().join("100.WORK/02.B/a.txt").is_file());
    let gone = ok(&call(&s, "delete", json!({ "relParts": ["100.WORK", "02.B"], "confirmNonEmpty": true })));
    assert_eq!(gone["relParts"], Value::Null);
    assert!(!dir.path().join("100.WORK/02.B").exists() && !in_tree(dir.path(), &["100.WORK", "02.B"]));
    assert_eq!(failed(&call(&s, "delete", json!({ "relParts": ["100.WORK", "05.Ghost"] })))["kind"], "stale");
    ok(&call(&s, "delete", json!({ "relParts": ["100.WORK", "05.Ghost"], "expectGhost": true })));
    assert!(!in_tree(dir.path(), &["100.WORK", "05.Ghost"]));
    let locked = failed(&call(&s, "delete", json!({ "relParts": ["100.WORK", "98.Rev"], "confirmNonEmpty": true })));
    assert_eq!(locked["kind"], "locked", "{locked}");
    assert!(dir.path().join("100.WORK/98.Rev").is_dir());
}

#[test]
fn metadata_tools_write_info_and_flags_and_repeat_safely() {
    let dir = work("mcp-w-meta");
    let s = writer(dir.path());
    ok(&call(&s, "set_type", json!({ "relParts": ["100.WORK", "02.B"], "type": "WKB", "status": "hold" })));
    assert!(get_type(&yaml(dir.path()), &rel(&["100.WORK", "02.B"])).starts_with("WKB"));
    ok(&call(&s, "set_status", json!({ "relParts": ["100.WORK", "02.B"], "status": "done" })));
    let owned = ok(&call(&s, "set_owner", json!({ "relParts": ["100.WORK", "02.B"], "owner": "Ana" })));
    assert_eq!(owned["relParts"], json!(["100.WORK", "02.B"]));
    let text = info(dir.path(), "100.WORK/02.B");
    assert!(text.contains("\"status\": \"done\"") && text.contains("\"owner\": \"Ana\""), "{text}");
    for _ in 0..2 {
        ok(&call(&s, "set_favorite", json!({ "relParts": ["100.WORK", "04.Wkb"], "on": true })));
    }
    ok(&call(&s, "set_important", json!({ "relParts": ["100.WORK", "04.Wkb"], "on": true })));
    ok(&call(&s, "hide", json!({ "relParts": ["100.WORK", "03.C"], "hidden": true })));
    let wkb = node(&s, &["100.WORK", "04.Wkb"]);
    assert_eq!((wkb["favorite"].clone(), wkb["important"].clone()), (json!(true), json!(true)), "{wkb}");
    assert!(failed(&call(&s, "get_node", json!({ "relParts": ["100.WORK", "03.C"] })))["kind"] == "notFound");
    ok(&call(&s, "hide", json!({ "relParts": ["100.WORK", "03.C"], "hidden": false })));
    assert_eq!(node(&s, &["100.WORK", "03.C"])["hidden"], false);
    ok(&call(&s, "set_board", json!({ "cards": [{ "relParts": ["100.WORK", "04.Wkb"], "column": "todo", "order": 0 }] })));
    assert!(info(dir.path(), "100.WORK/04.Wkb").contains("todo"));
    let not_wkb = failed(&call(&s, "set_owner", json!({ "relParts": ["100.WORK", "03.C"], "owner": "Bo" })));
    assert_eq!(not_wkb["kind"], "invalidPath");
}

#[test]
fn file_tools_rename_and_recycle_a_file() {
    let dir = work("mcp-w-files");
    let s = writer(dir.path());
    fs::write(dir.path().join("100.WORK/02.B/notes.txt"), "x").unwrap();
    ok(&call(&s, "rename_file", json!({ "relParts": ["100.WORK", "02.B", "notes.txt"], "name": "plan.txt" })));
    assert_eq!(names(&ok(&call(&s, "list_children", json!({ "relParts": ["100.WORK", "02.B"] })))["nodes"]), ["plan.txt"]);
    ok(&call(&s, "delete_file", json!({ "relParts": ["100.WORK", "02.B", "plan.txt"] })));
    assert!(!dir.path().join("100.WORK/02.B/plan.txt").exists());
}

#[test]
fn project_sort_previews_applies_and_undoes() {
    let dir = storage("mcp-w-sort", &["100.WORK/98.WORK"], "100.WORK:\n  type: NODE-L1\n  98.WORK: {type: REV}\n");
    for file in ["Alpha Beta Gamma v1.pdf", "Alpha Beta Gamma v2.pdf", "notes.md"] {
        fs::write(dir.path().join("100.WORK/98.WORK").join(file), "x").unwrap();
    }
    let s = writer(dir.path());
    let groups = ok(&call(&s, "preview_project_groups", json!({ "folder": ["100.WORK", "98.WORK"] })))["groups"].clone();
    assert_eq!(groups.as_array().unwrap().len(), 1, "{groups}");
    let plan = json!({ "name": "PJ_Alpha-Beta-Gamma", "files": groups[0]["files"], "owner": "Ana" });
    ok(&call(&s, "apply_project_groups", json!({ "folder": ["100.WORK", "98.WORK"], "plans": [plan] })));
    let board = dir.path().join("100.WORK/98.WORK/PJ_Alpha-Beta-Gamma");
    assert!(board.join("Alpha Beta Gamma v1.pdf").is_file());
    ok(&call(&s, "undo_project_sort", json!({})));
    assert!(!board.exists() && dir.path().join("100.WORK/98.WORK/Alpha Beta Gamma v1.pdf").is_file());
}

#[test]
fn refusals_come_back_as_fm_errors_and_change_nothing() {
    let dir = work("mcp-w-refuse");
    let s = writer(dir.path());
    let reserved = failed(&call(&s, "add_node", json!({ "parent": ["999.Settings"], "name": "X" })));
    assert_eq!(reserved, json!({ "kind": "reserved", "relParts": ["999.Settings"] }));
    assert_eq!(failed(&call(&s, "add_node", json!({ "parent": ["100.WORK", "98.Rev"], "name": "X" })))["kind"], "locked");
    dir.mkdir("100.WORK/01.A/01.Keep/Deep");
    let deep = failed(&call(&s, "set_type", json!({ "relParts": ["100.WORK", "01.A", "01.Keep", "Deep"], "type": "WKB" })));
    assert_eq!(deep, json!({ "kind": "depthLimit" }));
    assert_eq!(failed(&call(&s, "rename", json!({ "relParts": ["100.WORK", "02.B"], "name": "   " })))["kind"], "invalidName");
    assert!(dir.path().join("100.WORK/02.B").is_dir());
    let missing = exchange(&s, request(3, "tools/call", json!({ "name": "rename", "arguments": { "relParts": ["100.WORK", "02.B"] } })).as_bytes());
    assert_eq!(missing[0]["error"]["code"], -32602);
}

#[test]
fn a_busy_lock_is_retried_then_reported_and_a_stale_one_at_once() {
    let dir = work("mcp-w-lock");
    let mut s = writer(dir.path());
    s.retry = Retry { attempts: 1, backoff: Duration::ZERO };
    let held = StorageLock::acquire(dir.path()).unwrap();
    let busy = failed(&call(&s, "rename", json!({ "relParts": ["100.WORK", "02.B"], "name": "Bee" })));
    assert_eq!((busy["kind"].clone(), busy["op"].clone()), (json!("io"), json!("lock storage")));
    assert!(busy["message"].as_str().unwrap().contains("through 1 tries; nothing was changed"), "{busy}");
    assert!(dir.path().join("100.WORK/02.B").is_dir());
    s.retry = Retry { attempts: 2, backoff: Duration::from_millis(10) };
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(3500));
        drop(held);
    });
    ok(&call(&s, "rename", json!({ "relParts": ["100.WORK", "02.B"], "name": "Bee" })));
    releaser.join().unwrap();
    assert!(dir.path().join("100.WORK/02.Bee").is_dir());
    let lock = dir.path().join(".ant").join(LOCK_FILE);
    fs::File::create(&lock).unwrap().set_modified(SystemTime::now() - Duration::from_secs(600)).unwrap();
    let started = Instant::now();
    let stale = failed(&call(&s, "rename", json!({ "relParts": ["100.WORK", "02.Bee"], "name": "B" })));
    assert!(stale["message"].as_str().unwrap().contains("press Refresh") && started.elapsed() < Duration::from_secs(2), "{stale}");
}

#[test]
fn each_change_is_logged_for_the_open_window_and_a_refusal_is_not() {
    let dir = work("mcp-w-log");
    let s = writer(dir.path());
    let typed = ok(&call(&s, "set_type", json!({ "relParts": ["100.WORK", "02.B"], "type": "WKB", "status": "hold" })));
    let owned = ok(&call(&s, "set_owner", json!({ "relParts": ["100.WORK", "02.B"], "owner": "Ana" })));
    failed(&call(&s, "add_node", json!({ "parent": ["999.Settings"], "name": "X" })));
    ok(&call(&s, "get_node", json!({ "relParts": ["100.WORK", "02.B"] })));
    let log = changes::read(dir.path()).unwrap();
    assert_eq!(log.iter().map(|e| e.seq).collect::<Vec<_>>(), [1, 2]);
    for (entry, reply) in log.iter().zip([typed, owned]) {
        assert_eq!((json!(entry.levels), json!(entry.rescan)), (reply["levels"].clone(), reply["fullTree"].clone()));
        assert_eq!(entry.writer, std::process::id());
    }
}
