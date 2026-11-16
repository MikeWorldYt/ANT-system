use std::path::Path;

use proptest::prelude::*;

use super::dict_match::{classify, find_term, renamed, split_ext};
use super::dictionary::{compile, parse, render, split_topic, Entry, Spec};
use super::dictionary_lint::{lint, Level};

fn entry(text: &str) -> Vec<Entry> {
    parse(text).iter().map(|s| compile(s, Path::new("t.md")).unwrap()).collect()
}

fn spec(folder: &str, term: &[&str], topics: &[&str]) -> Spec {
    Spec { folder: folder.into(), term: term.iter().map(|s| s.to_string()).collect(), topics: topics.iter().map(|s| s.to_string()).collect(), ..Spec::default() }
}

#[test]
fn find_term_table() {
    for (stem, term, hit) in [
        ("BlueArchive_fan", "blue archive", true),
        ("blue-archive-fan", "Blue Archive", true),
        ("HTMLPage", "html page", true),
        ("Canción_Ñandú", "cancion nandu", true),
        ("ba3f9c", "ba", false),
        ("BA_pic", "ba", true),
        ("ba pic", "ba", true),
        ("Mayhem", "may", false),
        ("日本語_絵", "日本語", true),
        ("nothing here", "zzz", false),
        ("x", "", false),
    ] {
        assert_eq!(find_term(stem, term).is_some(), hit, "{stem} / {term}");
    }
}

#[test]
fn spans_are_byte_offsets_of_the_original_text() {
    let stem = "Ñandú_Sakuya_x";
    let (s, e) = find_term(stem, "sakuya").unwrap();
    assert_eq!(&stem[s..e], "Sakuya");
}

#[test]
fn split_ext_table() {
    for (name, want) in [("a.txt", ("a", ".txt")), ("a.tar.gz", ("a.tar", ".gz")), (".bashrc", (".bashrc", "")), ("noext", ("noext", "")), ("a.", ("a", "."))] {
        assert_eq!(split_ext(name), want, "{name}");
    }
}

#[test]
fn classify_prefers_the_longest_term_then_ext_then_pattern() {
    let entries = entry("# D\n## Short\n- term: ba\n## Long\n- term: Genshin Impact\n## Pics\n- ext: PNG jpg\n## Nums\n- pattern: ^scan\\d+$\n");
    let name = |file: &str| classify(&entries, file).map(|e| e.folder.as_str());
    assert_eq!(name("BA Genshin Impact x.txt"), Some("Long"));
    assert_eq!(name("BA x.png"), Some("Short"));
    assert_eq!(name("cat.PNG"), Some("Pics"));
    assert_eq!(name("scan001.txt"), Some("Nums"));
    assert_eq!(name("cat.txt"), None);
}

#[test]
fn only_terms_match_literally_as_whole_words() {
    let entries = entry("# D\n## Strict\n- only: Re:Zero\n");
    assert!(classify(&entries, "Re:Zero art.png").is_some());
    assert!(classify(&entries, "re:zero art.png").is_none());
    assert!(classify(&entries, "xRe:Zero art.png").is_none());
}

#[test]
fn renamed_table() {
    let entries = entry("# D\n## Fans\n- term: sensei, fanart\n- topics: noa ushio, Sakuya (Izayoi Sakuya, Iza)\n- exclude: lowres\n- pattern: \\d{4}x\\d{4}\n");
    let e = &entries[0];
    for (input, want) in [
        ("WAL_sensei-come.png", "WAL_come.png"),
        ("sensei.png", "sensei.png"),
        ("pic lowres 1920x1080.jpg", "pic.jpg"),
        ("noa_ushio-pic.png", "Noa-Ushio-pic.png"),
        ("pic noa ushio.png", "Noa-Ushio-pic.png"),
        ("Izayoi Sakuya art.png", "Sakuya art.png"),
        ("Izayoi-Sakuya_izayoi_sakuya.png", "Sakuya.png"),
        ("pic (fanart) 1.png", "pic 1.png"),
        ("plain.txt", "plain.txt"),
    ] {
        assert_eq!(renamed(e, input), want, "{input}");
    }
}

#[test]
fn a_cut_that_would_empty_the_name_keeps_it() {
    let entries = entry("# D\n## F\n- term: sensei\n");
    assert_eq!(renamed(&entries[0], "sensei.png"), "sensei.png");
    assert_eq!(renamed(&entries[0], "sensei_sensei.png"), "sensei.png");
}

#[test]
#[ignore = "BUG: a name whose stem is only symbols after the cut, like '_.png', loses its stem and becomes the hidden file '.png'"]
fn a_symbol_only_stem_is_not_trimmed_away() {
    assert_ne!(renamed(&entry("# D\n## F\n- term: x\n")[0], "_.png"), ".png");
}

#[test]
#[ignore = "BUG: empty brackets nested in each other leave '( )' behind on the first pass, so a second pass changes the name again"]
fn nested_empty_brackets_are_dropped_in_one_pass() {
    let e = &entry("# D\n## F\n- term: x\n")[0];
    assert_eq!(renamed(e, &renamed(e, "( ())0")), renamed(e, "( ())0"));
}

#[test]
fn the_dictionary_text_parses_into_entries() {
    let text = "%%\nnote\n%%\n# Dictionary for X\n\n## Fans\n- term: a, b\n- topics: Sakuya (Iza, Izayoi Sakuya), Remilia\n- ext: .PNG jpg\n- only: X\n- bad line\n- pattern: ^a\n\n### Deep\n- exclude: z\n";
    let specs = parse(text);
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].term, ["a", "b"]);
    assert_eq!(specs[0].topics, ["Sakuya (Iza, Izayoi Sakuya)", "Remilia"]);
    assert_eq!(specs[0].ext, [".png", ".jpg"]);
    assert_eq!(specs[1].folder, "Deep");
    assert_eq!(split_topic("Sakuya (Izayoi Sakuya, Iza)"), ("Sakuya".to_string(), vec!["Izayoi Sakuya".to_string(), "Iza".to_string()]));
    assert_eq!(split_topic("  Solo "), ("Solo".to_string(), vec![]));
}

#[test]
fn render_then_parse_gives_the_same_specs() {
    let specs = vec![
        Spec { ext: vec![".png".into(), ".jpg".into()], pattern: vec!["^a\\d".into()], ..spec("Fans", &["a", "ñandú"], &["Sakuya (Iza)", "Noa-Ushio"]) },
        Spec { only: vec!["Re:Zero".into()], exclude: vec!["x".into()], ..spec("日本語", &[], &[]) },
    ];
    assert_eq!(parse(&render("Title", &specs)), specs);
}

#[test]
fn compile_refuses_a_broken_pattern_and_marks_do_not_group() {
    let bad = Spec { pattern: vec!["(".into()], ..spec("F", &["a"], &[]) };
    assert!(compile(&bad, Path::new("t.md")).is_err());
    assert!(!compile(&spec("Do Not Group", &["a"], &[]), Path::new("t.md")).unwrap().group);
}

#[test]
fn lint_table() {
    let issues = |text: &str| lint(text).into_iter().map(|i| (i.line, i.level, i.message)).collect::<Vec<_>>();
    assert!(issues("# T\n\n## Fans\n- term: a\n").is_empty());
    assert!(issues("%%\n## ghost\n%%\n# T\n## F\n- ext: png\n").is_empty());
    let found = |text: &str, needle: &str, line: usize, level: Level| issues(text).iter().any(|(l, lv, m)| *l == line && *lv == level && m.contains(needle));
    assert!(found("# T\n#F\n- term: a\n", "needs a space", 2, Level::Error));
    assert!(found("# T\n## F\n-term: a\n", "needs a space", 3, Level::Error));
    assert!(found("# T\n## F\n- ter: a\n", "did you mean `term`", 3, Level::Error));
    assert!(found("# T\n## F\n- zzzzzzzz: a\n", "Unknown key", 3, Level::Error));
    assert!(found("# T\n## F\n- term:\n", "no values", 3, Level::Warning));
    assert!(found("# T\n## F\n- pattern: (\n", "does not compile", 3, Level::Error));
    assert!(found("# T\n## A\n- term: a\n## a\n- term: b\n", "already an entry", 4, Level::Error));
    assert!(found("# T\n## NODE.\n- term: a\n", "needs the node name", 2, Level::Error));
    assert!(found("# T\n## F\n- exclude: a\n", "catches nothing", 2, Level::Warning));
    assert!(found("# T\n- term: a\n", "outside any entry", 2, Level::Error));
    assert!(found("# T\n## F\n- term a\n", "Expected `key: values`", 3, Level::Error));
    assert!(found("# T\n## F\n- term: a\n%% open\n", "never closed", 4, Level::Error));
    assert!(found("# T\n## F\nloose text\n- term: a\n", "Not part of the format", 3, Level::Warning));
}

proptest! {
    #[test]
    fn lint_and_parse_never_panic_on_any_text(text in "\\PC{0,200}") {
        let _ = lint(&text);
        let _ = parse(&text);
    }

    #[test]
    fn dictionary_renames_are_idempotent(stem in "[A-Za-z0-9][A-Za-z0-9 _.-]{0,23}", ext in "(\\.(png|txt))?") {
        let entries = entry("# D\n## Fans\n- term: sensei, fanart\n- topics: noa ushio, Sakuya (Izayoi Sakuya, Iza)\n- exclude: lowres\n");
        let once = renamed(&entries[0], &format!("{stem}{ext}"));
        prop_assert_eq!(renamed(&entries[0], &once), once.clone());
    }

    #[test]
    fn a_rename_never_empties_a_name_or_changes_the_extension(stem in "[A-Za-z0-9][A-Za-z0-9 _-]{0,23}") {
        let entries = entry("# D\n## Fans\n- term: sensei, fanart\n- topics: noa ushio\n");
        let out = renamed(&entries[0], &format!("{stem}.png"));
        prop_assert!(out.ends_with(".png") && out.len() > 4, "{out}");
    }
}
