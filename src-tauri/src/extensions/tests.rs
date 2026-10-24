use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::time::{Duration, SystemTime};

use super::fetch::*;
use super::manifest::{check_info, main_parts, parse, valid_id};
use super::registry;
use super::staging::{expired, new_token, pick_root, valid_token};
use super::store::{self, parse_state, prune, state_json};
use super::tools::hash_from;
use crate::storage::db::Store;
use crate::test_support::TempDir;

const OK: &str = r#"{"id":"word-count","name":"Word count","version":"1.2.3","description":"Counts",
"author":"Ann","apiVersion":1,"main":"dist/main.js","permissions":["tree:read","tree:read"]}"#;

fn perms(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn with(field: &str, value: &str) -> String {
    let mut v: serde_yaml::Mapping = serde_yaml::from_str(OK).unwrap();
    v.insert(field.into(), serde_yaml::from_str(value).unwrap());
    serde_yaml::to_string(&v).unwrap()
}

#[test]
fn a_complete_manifest_passes() {
    let m = parse(OK).unwrap();
    assert_eq!((m.id.as_str(), m.main.as_str()), ("word-count", "dist/main.js"));
}

#[test]
fn ids_follow_the_rule() {
    for good in ["abc", "a-1", &"a".repeat(40)] {
        assert!(valid_id(good), "{good}");
    }
    for bad in ["ab", &"a".repeat(41), "Abc", "a_b", "a.b", "a b", "con", "nul", "../x"] {
        assert!(!valid_id(bad), "{bad}");
    }
}

#[test]
fn fields_are_checked() {
    let p = perms(&["panel:own"]);
    assert!(check_info("abc", "N", "0.0.1", "", "A", &p).is_ok());
    assert!(check_info("abc", "", "0.0.1", "", "A", &p).is_err());
    assert!(check_info("abc", &"n".repeat(61), "0.0.1", "", "A", &p).is_err());
    assert!(check_info("abc", "N\n", "0.0.1", "", "A", &p).is_err());
    for v in ["1.2", "1.2.3.4", "1.a.3", "v1.2.3", "1..3", ""] {
        assert!(check_info("abc", "N", v, "", "A", &p).is_err(), "{v}");
    }
    assert!(check_info("abc", "N", "1.0.0", &"d".repeat(201), "A", &p).is_err());
    assert!(check_info("abc", "N", "1.0.0", &"d".repeat(200), "A", &p).is_ok());
    assert!(check_info("abc", "N", "1.0.0", "", "", &p).is_err());
    assert!(check_info("abc", "N", "1.0.0", "", &"a".repeat(61), &p).is_err());
}

#[test]
fn permissions_must_be_known_and_present() {
    assert!(parse(&with("permissions", "[]")).unwrap_err().contains("no permissions"));
    assert!(parse(&with("permissions", "[\"net:fetch\"]")).unwrap_err().contains("unknown permission"));
    let mut v: serde_yaml::Mapping = serde_yaml::from_str(OK).unwrap();
    v.remove("permissions");
    assert!(parse(&serde_yaml::to_string(&v).unwrap()).is_err());
    assert!(parse(&with("permissions", "[\"tree:read\",\"menu:context\",\"panel:own\"]")).is_ok());
}

#[test]
fn api_version_and_missing_fields_reject() {
    assert!(parse(&with("apiVersion", "2")).is_err());
    assert!(parse("{}").is_err());
    assert!(parse("not json [").is_err());
}

#[test]
fn main_must_stay_inside_the_folder() {
    assert!(main_parts("main.js").is_some());
    assert!(main_parts("a/b/main.js").is_some());
    for bad in ["../main.js", "a/../main.js", "/main.js", "C:/main.js", "a\\main.js", "main.ts", "", "./main.js", "a//b.js"] {
        assert!(main_parts(bad).is_none(), "{bad}");
    }
}

#[test]
fn hashes_compare_as_lowercase_hex() {
    let a = "ab".repeat(32);
    assert!(valid_sha256(&a) && same_hash(&a, &a.to_uppercase()));
    assert!(!same_hash(&a, &"cd".repeat(32)));
    assert!(!valid_sha256(&"g".repeat(64)) && !valid_sha256("ab"));
    assert_eq!(hash_from(&format!("{}  file.zip\n", a.to_uppercase())).unwrap(), a);
    assert!(hash_from("").is_err());
}

#[test]
fn only_https_github_hosts_are_allowed() {
    assert!(allowed_url("https://github.com/a/b/archive.zip", &REGISTRY_HOSTS));
    assert!(allowed_url("https://raw.githubusercontent.com/a/b/main/x.zip", &REGISTRY_HOSTS));
    assert!(allowed_url("https://objects.githubusercontent.com/x?y=1", &REGISTRY_HOSTS));
    for bad in [
        "http://github.com/a",
        "https://evil.com/a",
        "https://github.com.evil.com/a",
        "https://user@github.com/a",
        "https://github.com:444/a",
        "https://github.com/a b",
        "https://github.com/\"a",
        "https://github.com/{a,b}",
        "https://",
    ] {
        assert!(!allowed_url(bad, &REGISTRY_HOSTS), "{bad}");
    }
    assert_eq!(host_of("https://api.github.com?x"), Some("api.github.com"));
    assert!(allowed_url("https://codeload.github.com/a/b/legacy.zip/main", &REPO_HOSTS));
}

#[test]
fn repository_addresses_are_strict() {
    let pair = |o: &str, r: &str| Some((o.to_string(), r.to_string()));
    assert_eq!(parse_repo_url("https://github.com/Mike/ant-ext"), pair("Mike", "ant-ext"));
    assert_eq!(parse_repo_url("https://github.com/Mike/ant.ext/"), pair("Mike", "ant.ext"));
    assert_eq!(parse_repo_url("https://github.com/Mike/ant_ext.git"), pair("Mike", "ant_ext"));
    for bad in [
        "http://github.com/a/b",
        "https://gitlab.com/a/b",
        "https://github.com/a",
        "https://github.com/a/b/tree/main",
        "https://github.com/a/b?x=1",
        "https://github.com/../b",
        "https://github.com/a/b#x",
        "https://github.com/a/.git",
        "https://github.com/a%20/b",
    ] {
        assert_eq!(parse_repo_url(bad), None, "{bad}");
    }
}

#[test]
fn curl_output_and_codes_are_read() {
    assert_eq!(split_effective(b"body\nmore\nhttps://x"), Some((&b"body\nmore"[..], "https://x")));
    assert_eq!(split_effective(b"\nhttps://x"), Some((&b""[..], "https://x")));
    assert_eq!(split_effective(b"none"), None);
    assert!(curl_message(Some(6)).contains("could not be reached"));
    assert!(curl_message(Some(63)).contains("larger"));
}

#[test]
fn tokens_and_staging_age() {
    let (a, b) = (new_token(), new_token());
    assert!(valid_token(&a) && a != b);
    for bad in ["", "0123456789abcde", "0123456789ABCDEF", "0123456789abcdeg", "../3456789abcdef"] {
        assert!(!valid_token(bad), "{bad}");
    }
    let now = SystemTime::now();
    assert!(expired(now - Duration::from_secs(3601), now));
    assert!(!expired(now - Duration::from_secs(60), now));
    assert!(!expired(now + Duration::from_secs(60), now));
}

#[test]
fn the_manifest_root_is_the_archive_root_or_its_single_folder() {
    let e = |list: &[(&str, bool)]| list.iter().map(|(n, d)| (n.to_string(), *d)).collect::<Vec<_>>();
    assert_eq!(pick_root(&e(&[("manifest.json", false), ("x", true)]), true), Some(None));
    assert_eq!(pick_root(&e(&[("repo-abc", true)]), false), Some(Some("repo-abc".to_string())));
    assert_eq!(pick_root(&e(&[("a", true), ("b", true)]), false), None);
    assert_eq!(pick_root(&e(&[("a.txt", false)]), false), None);
}

#[test]
fn state_round_trips_and_prunes() {
    let mut state = parse_state(r#"{"b-ext":{"enabled":true},"a-ext":{"enabled":false},"BAD":{"enabled":true}}"#);
    assert_eq!(state_json(&state), r#"{"a-ext":{"enabled":false},"b-ext":{"enabled":true}}"#);
    assert_eq!(parse_state(&state_json(&state)), state);
    assert!(parse_state("garbled [").is_empty());
    assert!(prune(&mut state, &HashSet::from(["b-ext"])));
    assert_eq!(state, BTreeMap::from([("b-ext".to_string(), true)]));
    assert!(!prune(&mut state, &HashSet::from(["b-ext", "c-ext"])));
}

#[test]
fn registry_entries_that_break_the_rules_are_skipped() {
    let entry = |id: &str, url: &str, sha: &str| {
        format!(r#"{{"id":"{id}","name":"N","version":"1.0.0","author":"A","permissions":["panel:own"],"url":"{url}","sha256":"{sha}"}}"#)
    };
    let sha = "a".repeat(64);
    let good = entry("good-one", "https://github.com/a/b/archive.zip", &sha);
    let text = format!(
        r#"{{"version":1,"extensions":[{good},{},{},{{"id":3}}]}}"#,
        entry("bad-url", "https://evil.com/x.zip", &sha),
        entry("bad-sha", "https://github.com/a/b/archive.zip", "abc"),
    );
    let items = registry::parse(&text).unwrap();
    assert_eq!(items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["good-one"]);
    assert!(registry::parse(r#"{"version":2,"extensions":[]}"#).is_err());
    assert!(registry::parse(r#"{ "version": 1, "extensions": [] }"#).unwrap().is_empty());
}

fn install(dir: &TempDir, folder: &str, manifest: &str) {
    let ext = store::extensions_dir(dir.path()).join(folder);
    fs::create_dir_all(ext.join("dist")).unwrap();
    fs::write(ext.join("manifest.json"), manifest).unwrap();
    fs::write(ext.join("dist/main.js"), "ant.panel.show();").unwrap();
}

#[test]
fn scanning_lists_ok_and_rejected_folders_and_drops_gone_state() {
    let dir = TempDir::new("ext-scan");
    install(&dir, "word-count", OK);
    install(&dir, "other-name", OK);
    install(&dir, ".staging", OK);
    Store::open_or_create(dir.path()).unwrap().set_setting("extensions", r#"{"word-count":{"enabled":true},"gone-ext":{"enabled":true}}"#).unwrap();
    let list = store::list(dir.path()).unwrap();
    assert_eq!(list.len(), 2);
    let ok = list.iter().find(|e| e.id == "word-count").unwrap();
    assert!(ok.enabled && ok.status == "ok" && ok.trust == "unvalidated");
    let bad = list.iter().find(|e| e.id == "other-name").unwrap();
    assert!(!bad.enabled && bad.status == "rejected" && bad.reason.as_deref().unwrap().contains("differs"));
    let saved = Store::open_existing(dir.path()).unwrap().unwrap().settings().unwrap();
    assert_eq!(saved["extensions"], r#"{"word-count":{"enabled":true}}"#);
}

#[test]
fn trust_comes_from_the_source_file() {
    let dir = TempDir::new("ext-trust");
    install(&dir, "word-count", OK);
    let source = store::extensions_dir(dir.path()).join("word-count/.source.json");
    fs::write(&source, r#"{"source":"registry","url":"https://github.com/x","sha256":"","trust":"validated"}"#).unwrap();
    assert_eq!(store::list(dir.path()).unwrap()[0].trust, "validated");
    fs::write(&source, "garbled [").unwrap();
    assert_eq!(store::list(dir.path()).unwrap()[0].trust, "unvalidated");
}

#[test]
fn a_missing_main_file_rejects() {
    let dir = TempDir::new("ext-main");
    install(&dir, "word-count", OK);
    fs::remove_file(store::extensions_dir(dir.path()).join("word-count/dist/main.js")).unwrap();
    assert_eq!(store::list(dir.path()).unwrap()[0].status, "rejected");
}

#[test]
fn enabling_reading_and_removing_follow_the_rules() {
    let dir = TempDir::new("ext-ops");
    let root = dir.path();
    install(&dir, "word-count", OK);
    install(&dir, "other-name", OK);
    assert!(store::read_entry(root, "word-count").is_err());
    assert!(store::set_enabled(root, "other-name", true).is_err());
    assert!(store::set_enabled(root, "missing-ext", true).is_err());
    assert!(store::set_enabled(root, "../x", true).is_err());
    store::set_enabled(root, "word-count", true).unwrap();
    assert_eq!(store::read_entry(root, "word-count").unwrap(), "ant.panel.show();");
    store::remove(root, "word-count").unwrap();
    assert!(!store::extensions_dir(root).join("word-count").exists());
    assert_eq!(Store::open_existing(root).unwrap().unwrap().settings().unwrap()["extensions"], "{}");
    assert!(store::remove(root, "word-count").is_err());
    assert!(store::remove(root, ".staging").is_err());
}

#[test]
fn entry_files_must_be_utf8() {
    let dir = TempDir::new("ext-utf8");
    install(&dir, "word-count", OK);
    fs::write(store::extensions_dir(dir.path()).join("word-count/dist/main.js"), [0xff, 0xfe]).unwrap();
    store::set_enabled(dir.path(), "word-count", true).unwrap();
    assert!(store::read_entry(dir.path(), "word-count").is_err());
}

#[test]
#[ignore = "downloads from GitHub"]
fn the_published_registry_parses() {
    assert!(registry::fetch().is_ok());
}
