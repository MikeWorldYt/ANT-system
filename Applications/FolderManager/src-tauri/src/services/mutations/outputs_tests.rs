// Deliveries checked on a throwaway storage, including outputs that try to leave the workboard.
use super::*;
use crate::infra::footprint_file::read_id;
use crate::test_support::{names, rel, storage, TempDir};

const WKB: &str = "100.WORK/PJ.006_Claude-2";
const OUT: &str = "100.WORK/00.Outcomes";
const DELIVERED: &str = "100.WORK/00.Outcomes/(26-11-01)_Claude-2";
const LINK: &str = "(26-11-01)_Claude-2.lnk";
const FOLDER_ID: &str = "wkb-lasting-id";

fn fixture(tag: &str) -> TempDir {
    let dir = storage(tag, &["100.WORK/PJ.006_Claude-2/dist", OUT], "100.WORK:\n  type: NODE-L1\n  PJ.006_Claude-2: {type: WKB-L2}\n  00.Outcomes: {type: OUT-L2}\n");
    std::fs::write(dir.path().join(WKB).join("dist/entregable.txt"), "done").unwrap();
    dir
}

fn workboard() -> Vec<String> {
    rel(&["100.WORK", "PJ.006_Claude-2"])
}

// What a real workboard holds in `.ant`: its status and owner, and its lasting id.
fn with_metadata(dir: &TempDir) {
    let board = dir.path().join(WKB);
    std::fs::create_dir_all(board.join(".ant")).unwrap();
    std::fs::write(board.join(".ant/info.json"), "{\"status\": \"done\", \"owner\": \"Ana\"}\n").unwrap();
    std::fs::write(board.join(".ant/dragpath.json"), format!("{{\"id\": \"{FOLDER_ID}\", \"flags\": []}}\n")).unwrap();
}

fn assert_metadata_kept(dir: &TempDir) {
    let board = dir.path().join(WKB);
    let info = read_info(&board).unwrap();
    assert_eq!(field_of(&info, &["owner"]), Some("Ana"));
    assert_eq!(read_id(&board).as_deref(), Some(FOLDER_ID));
    assert!(!dir.path().join(DELIVERED).join(".ant").exists());
}

fn store_output(dir: &TempDir, value: &str) {
    write_info(&dir.path().join(WKB), &[(&[OUTPUT_KEY][..], OUTPUT_KEY, Some(Value::String(value.to_string())))]).unwrap();
}

fn outside_folder(tag: &str) -> TempDir {
    let outside = TempDir::new(tag);
    std::fs::write(outside.path().join("keep.txt"), "mine").unwrap();
    outside
}

fn assert_no_output_set(result: R<Mutation>) {
    match result {
        Err(FmError::InvalidPath { reason, .. }) => assert_eq!(reason, "no output is set for this workboard"),
        Err(other) => panic!("{other:?}"),
        Ok(_) => panic!("the delivery went through"),
    }
}

#[cfg(windows)]
fn junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn delivering_moves_the_contents_of_the_output_folder_and_leaves_a_link_inside_it() {
    let dir = fixture("out-deliver");
    set_output(dir.path(), &workboard(), Some("/dist")).unwrap();
    deliver(dir.path(), &workboard(), "26-11-01", false).unwrap();
    assert!(dir.path().join(DELIVERED).join("entregable.txt").is_file());
    let left: Vec<String> = if cfg!(windows) { vec![LINK.to_string()] } else { Vec::new() };
    assert_eq!(names(&dir.path().join(WKB).join("dist")), left);
}

#[cfg(windows)]
#[test]
fn restoring_copies_the_delivery_back_and_removes_the_link() {
    let dir = fixture("out-restore");
    set_output(dir.path(), &workboard(), Some("dist")).unwrap();
    deliver(dir.path(), &workboard(), "26-11-01", false).unwrap();
    restore(dir.path(), &workboard()).unwrap();
    assert!(dir.path().join(WKB).join("dist/entregable.txt").is_file());
    assert!(dir.path().join(DELIVERED).join("entregable.txt").is_file());
    assert!(!dir.path().join(WKB).join("dist").join(LINK).exists());
}

#[test]
fn delivering_needs_an_output_a_date_and_a_path_inside_the_workboard() {
    let dir = fixture("out-refuse");
    assert!(deliver(dir.path(), &workboard(), "26-11-01", false).is_err());
    assert!(set_output(dir.path(), &workboard(), Some("../etc")).is_err());
    assert!(set_output(dir.path(), &workboard(), Some("missing")).is_err());
    set_output(dir.path(), &workboard(), Some("dist")).unwrap();
    assert!(matches!(deliver(dir.path(), &workboard(), "Nov 1", false), Err(FmError::InvalidName { .. })));
}

#[test]
fn a_file_or_a_nested_folder_inside_the_workboard_is_still_a_valid_output() {
    let dir = fixture("out-valid");
    set_output(dir.path(), &workboard(), Some("dist\\entregable.txt")).unwrap();
    deliver(dir.path(), &workboard(), "26-11-01", false).unwrap();
    assert!(dir.path().join(DELIVERED).join("entregable.txt").is_file());
    dir.mkdir(&format!("{WKB}/dist/final v2"));
    std::fs::write(dir.path().join(WKB).join("dist/final v2/report.txt"), "done").unwrap();
    set_output(dir.path(), &workboard(), Some("dist/final v2")).unwrap();
    deliver(dir.path(), &workboard(), "26-11-02", false).unwrap();
    assert!(dir.path().join("100.WORK/00.Outcomes/(26-11-02)_Claude-2/report.txt").is_file());
}

#[test]
fn clean_output_refuses_drives_and_app_folders_in_any_part() {
    let rel = workboard();
    for raw in ["C:/x", "C:x", "D:\\x", ".ant/x", ".ANT\\x", "dist/.ant", "dist/C:/x", "\\\\?\\C:\\x", "dist/x:stream"] {
        assert!(clean_output(&rel, raw).is_err(), "{raw}");
    }
}

#[test]
fn clean_output_keeps_ordinary_paths_relative() {
    assert_eq!(clean_output(&workboard(), "\\dist\\final v2/").unwrap(), "dist/final v2");
}

#[test]
fn a_crafted_output_counts_as_unset_and_leaves_the_outside_folder_unchanged() {
    let dir = fixture("out-crafted");
    let outside = outside_folder("out-crafted-outside");
    for value in [outside.path().to_string_lossy().into_owned(), outside.path().to_string_lossy().replace('\\', "/")] {
        store_output(&dir, &value);
        assert_no_output_set(deliver(dir.path(), &workboard(), "26-11-01", false));
        assert_eq!(names(outside.path()), ["keep.txt"]);
        assert!(names(&dir.path().join(OUT)).is_empty());
    }
}

#[test]
fn an_output_inside_the_app_folder_counts_as_unset() {
    let dir = fixture("out-crafted-ant");
    with_metadata(&dir);
    store_output(&dir, ".ant");
    assert_no_output_set(deliver(dir.path(), &workboard(), "26-11-01", false));
    assert_metadata_kept(&dir);
}

#[test]
fn restoring_never_writes_to_a_crafted_output_folder() {
    let dir = fixture("out-crafted-restore");
    let outside = outside_folder("out-crafted-restore-outside");
    std::fs::write(outside.path().join(LINK), "").unwrap();
    dir.mkdir(DELIVERED);
    std::fs::write(dir.path().join(DELIVERED).join("delivered.txt"), "done").unwrap();
    store_output(&dir, &outside.path().to_string_lossy());
    restore(dir.path(), &workboard()).unwrap();
    let mut expected = vec![LINK, "keep.txt"];
    expected.sort_unstable();
    assert_eq!(names(outside.path()), expected);
}

#[cfg(windows)]
#[test]
fn an_output_behind_a_junction_counts_as_unset() {
    let dir = fixture("out-junction");
    let outside = outside_folder("out-junction-outside");
    // `mklink` reads a forward slash as a switch, so the path is rebuilt with native separators.
    let link: PathBuf = dir.path().join(WKB).join("linked").components().collect();
    if !junction(&link, outside.path()) {
        eprintln!("skipped: could not create a junction");
        return;
    }
    assert!(set_output(dir.path(), &workboard(), Some("linked")).is_err());
    for value in ["linked", "linked/keep.txt"] {
        store_output(&dir, value);
        assert_no_output_set(deliver(dir.path(), &workboard(), "26-11-01", false));
        assert_eq!(names(outside.path()), ["keep.txt"]);
    }
    std::fs::remove_dir(&link).unwrap();
}

#[test]
fn moving_everything_leaves_the_workboard_metadata_in_place() {
    let dir = fixture("out-all");
    with_metadata(&dir);
    deliver(dir.path(), &workboard(), "26-11-01", true).unwrap();
    assert!(dir.path().join(DELIVERED).join("dist/entregable.txt").is_file());
    assert_metadata_kept(&dir);
}

#[test]
fn delivering_an_output_folder_keeps_its_own_metadata() {
    let dir = fixture("out-folder-meta");
    let dist = dir.path().join(WKB).join("dist");
    std::fs::create_dir_all(dist.join(".ant")).unwrap();
    std::fs::write(dist.join(".ant/dragpath.json"), format!("{{\"id\": \"{FOLDER_ID}\", \"flags\": []}}\n")).unwrap();
    set_output(dir.path(), &workboard(), Some("dist")).unwrap();
    deliver(dir.path(), &workboard(), "26-11-01", false).unwrap();
    assert_eq!(read_id(&dist).as_deref(), Some(FOLDER_ID));
    assert!(!dir.path().join(DELIVERED).join(".ant").exists());
}

#[cfg(windows)]
#[test]
fn a_full_delivery_and_restore_keep_the_workboard_metadata() {
    let dir = fixture("out-all-restore");
    with_metadata(&dir);
    deliver(dir.path(), &workboard(), "26-11-01", true).unwrap();
    let restored = restore(dir.path(), &workboard()).unwrap();
    assert!(restored.warnings.is_empty(), "{:?}", restored.warnings);
    assert!(dir.path().join(WKB).join("dist/entregable.txt").is_file());
    assert!(!dir.path().join(WKB).join(LINK).exists());
    assert_metadata_kept(&dir);
}

#[test]
fn restoring_skips_metadata_left_in_an_older_delivery() {
    let dir = fixture("out-old-delivery");
    dir.mkdir(&format!("{DELIVERED}/.ant"));
    std::fs::write(dir.path().join(DELIVERED).join(".ant/dragpath.json"), "{\"id\": \"old\", \"flags\": []}\n").unwrap();
    std::fs::write(dir.path().join(DELIVERED).join("delivered.txt"), "done").unwrap();
    std::fs::write(dir.path().join(WKB).join(LINK), "").unwrap();
    restore(dir.path(), &workboard()).unwrap();
    assert!(dir.path().join(WKB).join("delivered.txt").is_file());
    assert_eq!(read_id(&dir.path().join(WKB)), None);
}
