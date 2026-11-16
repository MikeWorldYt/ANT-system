use std::collections::HashSet;
use std::fs;

use proptest::prelude::*;

use super::{plan, Picks, PlannedFile};
use crate::infra::fs_safe::validate_component;
use crate::test_support::{rel, TempDir};

const FOLDER: &str = "REV.Test";
const DICT: &str = "# D\n## Fans\n- term: sensei\n- topics: noa ushio\n## Do not group\n- term: keep\n";

fn storage(files: &[&str], dict: Option<&str>) -> TempDir {
    let dir = TempDir::new("plan");
    dir.mkdir(FOLDER);
    for f in files {
        fs::write(dir.path().join(FOLDER).join(f), "x").unwrap();
    }
    if let Some(text) = dict {
        dir.mkdir(&format!("{FOLDER}/.ant"));
        fs::write(dir.path().join(FOLDER).join(".ant/dictionary.md"), text).unwrap();
    }
    dir
}

fn run(dir: &TempDir, only: &[&str], picks: Picks) -> Vec<PlannedFile> {
    let only: Vec<String> = only.iter().map(|s| s.to_string()).collect();
    plan(dir.path(), &rel(&[FOLDER]), &only, picks).unwrap()
}

fn names(rows: &[PlannedFile]) -> Vec<(&str, &str)> {
    rows.iter().map(|r| (r.file.as_str(), r.name.as_str())).collect()
}

fn reduce() -> Picks {
    Picks { reduce: true, ..Picks::default() }
}

fn group() -> Picks {
    Picks { group: true, ..Picks::default() }
}

#[test]
fn a_missing_or_empty_folder_plans_nothing() {
    let dir = storage(&[], None);
    assert!(run(&dir, &[], reduce()).is_empty());
    assert!(plan(dir.path(), &rel(&["REV.Gone"]), &[], reduce()).unwrap().is_empty());
}

#[test]
fn nothing_is_renamed_without_picks() {
    let dir = storage(&["image.png", "a_n.png"], None);
    assert!(run(&dir, &[], Picks::default()).iter().all(|r| r.name == r.file && r.folder.is_none() && r.project.is_none()));
}

#[test]
fn reduced_names_that_collide_get_a_counter() {
    let dir = storage(&["a.png", "a_n.png", "a_n .png"], None);
    assert_eq!(names(&run(&dir, &[], reduce())), [("a.png", "a.png"), ("a_n .png", "a (2).png"), ("a_n.png", "a (3).png")]);
}

#[test]
fn temporary_copies_are_never_touched() {
    let dir = storage(&["~$report.docx", "image.png"], None);
    let rows = run(&dir, &[], reduce());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].file, "image.png");
}

#[test]
fn removing_a_date_wins_over_adding_one() {
    let dir = storage(&["20240905_foto.jpg"], None);
    assert_eq!(run(&dir, &[], Picks { remove_date: true, add_date: true, ..Picks::default() })[0].name, "foto.jpg");
    let added = run(&dir, &[], Picks { add_date: true, ..Picks::default() });
    assert_eq!(added[0].name, "(24-09-05)_foto.jpg");
}

#[test]
fn symbols_go_last_and_stay_unique() {
    let dir = storage(&["a (1).png", "a 1.png"], None);
    assert_eq!(names(&run(&dir, &[], Picks { remove_symbols: true, ..Picks::default() })), [("a (1).png", "a 1 (2).png"), ("a 1.png", "a 1.png")]);
}

#[test]
fn only_narrows_the_rows_but_not_the_collision_check() {
    let dir = storage(&["a.png", "a_n.png"], None);
    assert_eq!(names(&run(&dir, &["a_n.png"], reduce())), [("a_n.png", "a (2).png")]);
}

#[test]
fn the_dictionary_moves_files_into_subfolders_without_landing_on_a_name() {
    let dir = storage(&["sensei-aaa.png", "sensei-bbb.png", "noa ushio-pic.png", "keep-pic.png", "other.txt"], Some(DICT));
    dir.mkdir(&format!("{FOLDER}/Fans"));
    fs::write(dir.path().join(FOLDER).join("Fans/aaa.png"), "x").unwrap();
    let rows = run(&dir, &[], group());
    let row = |file: &str| rows.iter().find(|r| r.file == file).unwrap();
    assert_eq!((row("sensei-aaa.png").folder.as_deref(), row("sensei-aaa.png").name.as_str()), (Some("Fans"), "aaa (2).png"));
    assert_eq!((row("sensei-bbb.png").folder.as_deref(), row("sensei-bbb.png").name.as_str()), (Some("Fans"), "bbb.png"));
    assert_eq!((row("noa ushio-pic.png").folder.as_deref(), row("noa ushio-pic.png").name.as_str()), (Some("Fans"), "Noa-Ushio-pic.png"));
    assert_eq!((row("keep-pic.png").folder.as_deref(), row("keep-pic.png").name.as_str()), (None, "pic.png"));
    assert_eq!(row("other.txt").folder, None);
}

#[test]
fn do_not_group_tidies_the_name_and_leaves_the_file() {
    let dir = storage(&["keep-pic.png", "pic.png"], Some("# D\n## Do not group\n- term: keep\n"));
    let rows = run(&dir, &[], group());
    assert!(rows.iter().all(|r| r.folder.is_none()));
    assert_eq!(names(&rows), [("keep-pic.png", "pic (2).png"), ("pic.png", "pic.png")]);
}

#[test]
fn grouped_files_are_left_out_of_the_project_sort() {
    let dir = storage(&["sensei-aaa.png"], Some(DICT));
    let rows = run(&dir, &[], Picks { group: true, sort: true, ..Picks::default() });
    assert!(rows[0].folder.is_some() && rows[0].project.is_none());
}

#[test]
fn removing_symbols_never_lands_on_a_file_already_in_the_subfolder() {
    let dir = storage(&["sensei-a (b).png"], Some(DICT));
    dir.mkdir(&format!("{FOLDER}/Fans"));
    fs::write(dir.path().join(FOLDER).join("Fans/a b.png"), "x").unwrap();
    let rows = run(&dir, &[], Picks { group: true, remove_symbols: true, ..Picks::default() });
    assert_ne!(rows[0].name, "a b.png");
}

#[test]
fn a_dictionary_heading_cannot_point_outside_the_folder() {
    let dir = storage(&["sensei-a.png"], Some("# D\n## ..\n- term: sensei\n"));
    let rows = run(&dir, &[], group());
    assert!(rows.iter().all(|r| r.folder.as_deref().is_none_or(|f| validate_component(f).is_ok())));
}

fn file_name() -> impl Strategy<Value = String> {
    ("[A-Za-z0-9][A-Za-z0-9 _()\\[\\]-]{0,8}[A-Za-z0-9]", "(\\.(png|txt|pdf))?", prop::sample::select(vec!["", "sensei-", "keep_", "noa ushio ", "20240905_", "image", "~"])).prop_map(|(stem, ext, lead)| format!("f{lead}{stem}{ext}"))
}

fn files() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(file_name(), 1..8).prop_map(|v| {
        let mut seen = HashSet::new();
        v.into_iter().filter(|f| seen.insert(f.to_lowercase())).collect()
    })
}

fn picks() -> impl Strategy<Value = Picks> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(reduce, sort, group, add_date, remove_date, remove_symbols)| Picks { reduce, sort, group, add_date, remove_date, remove_symbols })
}

fn assert_safe(rows: &[PlannedFile], existing: &HashSet<(Option<String>, String)>) -> Result<(), TestCaseError> {
    let mut seen = HashSet::new();
    for r in rows {
        prop_assert!(validate_component(&r.name).is_ok(), "{:?}", r);
        for part in r.folder.iter().flat_map(|f| f.split('/')) {
            prop_assert!(validate_component(part).is_ok(), "{:?}", r);
        }
        let key = (r.folder.clone(), r.name.to_lowercase());
        prop_assert!(seen.insert(key.clone()), "two files land on {key:?}");
        if r.folder.is_some() || r.name != r.file {
            prop_assert!(!existing.contains(&key) || (r.folder.is_none() && r.name == r.file), "{r:?} lands on an existing file");
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn a_plan_never_collides_and_stays_inside_the_folder(names in files(), mut picks in picks(), half in any::<bool>()) {
        picks.remove_symbols &= !picks.group;
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let dir = storage(&refs, Some(DICT));
        dir.mkdir(&format!("{FOLDER}/Fans"));
        fs::write(dir.path().join(FOLDER).join("Fans/aaa.png"), "x").unwrap();
        let only: Vec<&str> = if half { refs.iter().step_by(2).copied().collect() } else { Vec::new() };
        let rows = run(&dir, &only, picks);
        let mut existing: HashSet<(Option<String>, String)> = names.iter().filter(|n| !n.starts_with("f~") && !only.is_empty() && !only.contains(&n.as_str())).map(|n| (None, n.to_lowercase())).collect();
        existing.insert((Some("Fans".to_string()), "aaa.png".to_string()));
        assert_safe(&rows, &existing)?;
        for r in &rows {
            prop_assert!(!r.file.starts_with('~') && names.contains(&r.file));
        }
    }

    #[test]
    fn the_same_folder_plans_the_same_rows(names in files(), picks in picks()) {
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let dir = storage(&refs, Some(DICT));
        let picks = Picks { add_date: false, ..picks };
        let (a, b) = (run(&dir, &[], picks), run(&dir, &[], picks));
        prop_assert_eq!(names_of(&a), names_of(&b));
    }
}

fn names_of(rows: &[PlannedFile]) -> Vec<(String, String, Option<String>)> {
    rows.iter().map(|r| (r.file.clone(), r.name.clone(), r.folder.clone())).collect()
}
