use std::fs;

use super::shortcut_plan;
use crate::error::FmError;
use crate::test_support::{academic, rel, yaml, TempDir};

#[test]
fn the_shortcut_points_at_the_folder_and_numbers_a_taken_name() {
    let storage = academic("share-plan");
    let out = TempDir::new("share-out");
    let bio = rel(&["100.ACADEMIC", "02.Bio"]);
    let (target, link) = shortcut_plan(storage.path(), &yaml(storage.path()), &bio, out.path()).unwrap();
    assert_eq!(target, storage.path().join("100.ACADEMIC").join("02.Bio"));
    assert_eq!(link, out.path().join("02.Bio"));
    #[cfg(windows)]
    {
        fs::write(out.path().join("02.Bio.lnk"), "x").unwrap();
        fs::write(out.path().join("02.Bio (2).lnk"), "x").unwrap();
        let (_, link) = shortcut_plan(storage.path(), &yaml(storage.path()), &bio, out.path()).unwrap();
        assert_eq!(link, out.path().join("02.Bio (3)"));
    }
}

#[test]
fn reserved_and_missing_folders_get_no_shortcut() {
    let storage = academic("share-refused");
    let out = TempDir::new("share-refused-out");
    let data = yaml(storage.path());
    let reserved = shortcut_plan(storage.path(), &data, &rel(&["999.Settings"]), out.path()).unwrap_err();
    assert!(matches!(reserved, FmError::Reserved { .. }), "{reserved:?}");
    let ghost = shortcut_plan(storage.path(), &data, &rel(&["100.ACADEMIC", "03.Ghost"]), out.path()).unwrap_err();
    assert!(matches!(ghost, FmError::GhostNotAllowed { .. }), "{ghost:?}");
}

#[test]
fn the_destination_must_be_an_existing_folder_outside_the_storage() {
    let storage = academic("share-dest");
    let out = TempDir::new("share-dest-out");
    fs::write(out.path().join("file.txt"), "x").unwrap();
    let data = yaml(storage.path());
    let bio = rel(&["100.ACADEMIC", "02.Bio"]);
    for dir in [out.path().join("missing"), out.path().join("file.txt"), storage.path().join("100.ACADEMIC")] {
        let err = shortcut_plan(storage.path(), &data, &bio, &dir).unwrap_err();
        assert!(matches!(err, FmError::Io { .. }), "{}: {err:?}", dir.display());
    }
}
