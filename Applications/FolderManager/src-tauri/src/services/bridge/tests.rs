// The bridge over a real loopback socket, with a stand-in for the webview, and its file, parsing and refusals on their own.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::client::{send, NOT_RUNNING};
use super::endpoint::{self, Endpoint};
use super::server::{answer, bind, parse, serve};
use super::{Request, UiCall};
use crate::test_support::TempDir;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn line(value: Value) -> Vec<u8> {
    (value.to_string() + "\n").into_bytes()
}

// The stand-in answers each call with the call itself, and keeps what it was asked.
fn running(open: PathBuf, dir: &Path) -> (PathBuf, Arc<Mutex<Vec<UiCall>>>) {
    let (listener, port) = bind().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    std::thread::spawn(move || {
        serve(listener, TOKEN.to_string(), move || Some(open.clone()), move |call| {
            seen.lock().unwrap().push(call.clone());
            Ok(serde_json::to_value(call).unwrap())
        })
    });
    let file = endpoint::file(dir, Path::new("C:\\Apps\\fm.exe"));
    endpoint::write(&file, &Endpoint { port, token: TOKEN.to_string() }).unwrap();
    (file, calls)
}

#[test]
fn the_endpoint_file_reads_back_and_is_removed_only_by_its_own_session() {
    let dir = TempDir::new("bridge-file");
    let f = endpoint::file(dir.path(), Path::new("C:\\Apps\\fm.exe"));
    assert!(f.starts_with(dir.path().join("ui-bridge")));
    assert_ne!(f, endpoint::file(dir.path(), Path::new("C:\\Other\\fm.exe")));
    assert_eq!(endpoint::read(&f), None);
    let written = Endpoint { port: 4242, token: TOKEN.to_string() };
    endpoint::write(&f, &written).unwrap();
    assert_eq!(endpoint::read(&f), Some(written));
    endpoint::remove(&f, "another session");
    assert!(f.exists());
    endpoint::remove(&f, TOKEN);
    assert!(!f.exists());
    std::fs::write(&f, "not json").unwrap();
    assert_eq!(endpoint::read(&f), None);
}

#[test]
fn a_new_token_is_long_random_hex() {
    let (a, b) = (endpoint::new_token().unwrap(), endpoint::new_token().unwrap());
    assert!(a.len() == 64 && a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b);
}

#[test]
fn a_request_parses_only_with_the_session_token() {
    let call = json!({ "action": "reveal", "relParts": ["100.WORK"] });
    let good = parse(&line(json!({ "token": TOKEN, "root": "C:\\S", "call": call })), TOKEN).unwrap().unwrap();
    assert_eq!((good.root, good.call), (PathBuf::from("C:\\S"), UiCall::Reveal { rel_parts: vec!["100.WORK".to_string()] }));
    let short = &TOKEN[..63];
    for ignored in [json!({ "root": "C:\\S", "call": call }), json!({ "token": short, "root": "C:\\S", "call": call }), json!({ "token": 7, "call": call }), json!({ "token": TOKEN.replace('0', "1"), "call": call })] {
        assert!(parse(&line(ignored.clone()), TOKEN).is_none(), "{ignored}");
    }
    assert!(parse(b"{not json\n", TOKEN).is_none());
    for bad in [json!({ "token": TOKEN, "root": "C:\\S", "call": { "action": "rmRf" } }), json!({ "token": TOKEN, "root": "C:\\S", "call": { "action": "reveal", "relParts": [], "extra": 1 } }), json!({ "token": TOKEN, "call": { "action": "getState" } })] {
        assert!(parse(&line(bad.clone()), TOKEN).unwrap().is_err(), "{bad}");
    }
    let open = parse(&line(json!({ "token": TOKEN, "root": "C:\\S", "call": { "action": "openTab", "relParts": null } })), TOKEN).unwrap().unwrap();
    assert_eq!(open.call, UiCall::OpenTab { rel_parts: None });
}

#[test]
fn a_request_for_another_storage_is_refused_before_the_app_runs_it() {
    let dir = TempDir::new("bridge-root");
    dir.mkdir("Store");
    let open = dir.path().join("Store");
    let run = |call: UiCall| Ok(serde_json::to_value(call).unwrap());
    let request = |root: PathBuf| Request { token: TOKEN.to_string(), root, call: UiCall::GetState };
    assert_eq!(answer(request(dir.path().join("Store/../Store")), Some(open.clone()), &run), Ok(json!({ "action": "getState" })));
    let other = answer(request(dir.path().to_path_buf()), Some(open.clone()), &run).unwrap_err();
    assert!(other.contains("another storage open"), "{other}");
    assert!(answer(request(open), None, &run).unwrap_err().contains("no storage open"));
}

#[test]
fn a_running_app_answers_over_loopback_and_a_wrong_token_gets_no_answer() {
    let dir = TempDir::new("bridge-live");
    let root = dir.path().to_path_buf();
    let (file, calls) = running(root.clone(), dir.path());
    let view = UiCall::SetView { view: "tree".to_string() };
    assert_eq!(send(&file, &root, view.clone()), Ok(json!({ "action": "setView", "view": "tree" })));
    let port = endpoint::read(&file).unwrap().port;
    endpoint::write(&file, &Endpoint { port, token: TOKEN.replace('a', "b") }).unwrap();
    let refused = send(&file, &root, UiCall::GetState).unwrap_err();
    assert!(refused.contains("did not accept"), "{refused}");
    assert_eq!(*calls.lock().unwrap(), [view]);
}

#[test]
fn without_a_running_app_every_call_says_so() {
    let dir = TempDir::new("bridge-closed");
    let file = endpoint::file(dir.path(), Path::new("C:\\Apps\\fm.exe"));
    assert_eq!(send(&file, dir.path(), UiCall::GetState), Err(NOT_RUNNING.to_string()));
    let (listener, port) = bind().unwrap();
    drop(listener);
    endpoint::write(&file, &Endpoint { port, token: TOKEN.to_string() }).unwrap();
    assert_eq!(send(&file, dir.path(), UiCall::GetState), Err(NOT_RUNNING.to_string()));
}
