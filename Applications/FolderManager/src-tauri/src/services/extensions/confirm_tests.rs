use std::fs;
use std::path::PathBuf;

use super::store;
use crate::infra::storage::db::Store;
use crate::test_support::TempDir;

const MANIFEST: &str = r#"{"id":"word-count","name":"Word count","version":"1.2.3","author":"Ann","apiVersion":1,"main":"main.js","permissions":["tree:read"]}"#;
const FOREIGN_STATE: &str = r#"{"word-count":{"enabled":true}}"#;
const CONF: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"));
const SANDBOX_TS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/extensions/sandbox.ts"));

fn install(dir: &TempDir) {
    let ext = store::extensions_dir(dir.path()).join("word-count");
    fs::create_dir_all(&ext).unwrap();
    fs::write(ext.join("manifest.json"), MANIFEST).unwrap();
    fs::write(ext.join("main.js"), "ant.tree.get();").unwrap();
}

fn foreign_storage(name: &str) -> TempDir {
    let dir = TempDir::new(name);
    install(&dir);
    Store::open_or_create(dir.path()).unwrap().set_setting("extensions", FOREIGN_STATE).unwrap();
    dir
}

fn machine_file(dir: &TempDir) -> PathBuf {
    dir.path().join("machine").join("confirmed.txt")
}

#[test]
fn a_foreign_enabled_state_leaves_the_extension_off() {
    let storage = foreign_storage("ext-foreign");
    let machine = TempDir::new("ext-foreign-machine");
    let file = machine_file(&machine);

    let listed = store::list_with(storage.path(), &file).unwrap();
    assert!(!listed[0].enabled, "ant.db alone must not turn an extension on");
    assert!(listed[0].needs_confirmation);
    assert!(store::read_entry_with(storage.path(), "word-count", &file).unwrap_err().to_string().contains("disabled"));

    let saved = Store::open_existing(storage.path()).unwrap().unwrap().settings().unwrap();
    assert_eq!(saved["extensions"], FOREIGN_STATE, "ant.db keeps its format and its value");
}

#[test]
fn one_confirmation_turns_an_extension_on_for_this_storage_and_code_only() {
    let storage = foreign_storage("ext-confirmed");
    let machine = TempDir::new("ext-confirmed-machine");
    let file = machine_file(&machine);

    store::set_enabled_with(storage.path(), "word-count", true, &file).unwrap();
    let listed = store::list_with(storage.path(), &file).unwrap();
    assert!(listed[0].enabled && !listed[0].needs_confirmation);
    assert_eq!(store::read_entry_with(storage.path(), "word-count", &file).unwrap(), "ant.tree.get();");

    let other = foreign_storage("ext-confirmed-other");
    assert!(!store::list_with(other.path(), &file).unwrap()[0].enabled, "same id and code in another storage");

    fs::write(store::extensions_dir(storage.path()).join("word-count/main.js"), "changed();").unwrap();
    let changed = store::list_with(storage.path(), &file).unwrap();
    assert!(!changed[0].enabled && changed[0].needs_confirmation, "changed code needs a new confirmation");
    assert!(store::read_entry_with(storage.path(), "word-count", &file).is_err());
}

#[test]
fn turning_off_or_removing_drops_the_confirmation() {
    let storage = foreign_storage("ext-dropped");
    let machine = TempDir::new("ext-dropped-machine");
    let file = machine_file(&machine);

    store::set_enabled_with(storage.path(), "word-count", true, &file).unwrap();
    store::set_enabled_with(storage.path(), "word-count", false, &file).unwrap();
    Store::open_or_create(storage.path()).unwrap().set_setting("extensions", FOREIGN_STATE).unwrap();
    assert!(!store::list_with(storage.path(), &file).unwrap()[0].enabled);

    store::set_enabled_with(storage.path(), "word-count", true, &file).unwrap();
    store::remove_with(storage.path(), "word-count", &file).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "");
}

#[test]
fn webrtc_is_blocked_in_the_webview_and_the_frame() {
    let conf: serde_yaml::Value = serde_yaml::from_str(CONF).unwrap();
    let args = conf["app"]["windows"][0]["additionalBrowserArgs"].as_str().expect("additionalBrowserArgs must be set");
    assert!(args.contains("--blink-settings=edgeWebRTCEnabled=false"), "{args}");
    assert!(args.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"), "wry's defaults must stay: {args}");
    for name in ["\"RTCPeerConnection\"", "\"webkitRTCPeerConnection\""] {
        assert!(SANDBOX_TS.contains(name), "{name} must be removed before the extension code runs");
    }
}
