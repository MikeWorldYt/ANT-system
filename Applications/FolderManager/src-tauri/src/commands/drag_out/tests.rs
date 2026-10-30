use super::{drag_plan, ALLOWED_EFFECTS};
use crate::error::FmError;
use crate::test_support::{academic, rel, yaml};

#[test]
fn the_drag_offers_copy_and_link_but_never_move() {
    assert_eq!(ALLOWED_EFFECTS, 1 | 4);
    assert_eq!(ALLOWED_EFFECTS & 2, 0);
    #[cfg(windows)]
    {
        use windows::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK};
        assert_eq!(ALLOWED_EFFECTS, (DROPEFFECT_COPY | DROPEFFECT_LINK).0);
    }
}

#[test]
fn an_existing_folder_drags_its_absolute_path() {
    let storage = academic("drag-out-plan");
    let bio = rel(&["100.ACADEMIC", "02.Bio"]);
    let path = drag_plan(storage.path(), &yaml(storage.path()), &bio).unwrap();
    assert_eq!(path, storage.path().join("100.ACADEMIC").join("02.Bio"));
}

#[test]
fn reserved_and_missing_folders_cannot_be_dragged_out() {
    let storage = academic("drag-out-refused");
    let data = yaml(storage.path());
    let reserved = drag_plan(storage.path(), &data, &rel(&["999.Settings"])).unwrap_err();
    assert!(matches!(reserved, FmError::Reserved { .. }), "{reserved:?}");
    let ghost = drag_plan(storage.path(), &data, &rel(&["100.ACADEMIC", "03.Ghost"])).unwrap_err();
    assert!(matches!(ghost, FmError::GhostNotAllowed { .. }), "{ghost:?}");
}

#[cfg(windows)]
#[test]
fn the_dropfiles_block_lists_wide_paths_after_its_header() {
    use std::path::PathBuf;
    use windows_sys::Win32::UI::Shell::DROPFILES;

    use crate::commands::clipboard::hdrop_bytes;

    let paths = [PathBuf::from(r"C:\a"), PathBuf::from(r"D:\bé")];
    let bytes = hdrop_bytes(&paths);
    let header = size_of::<DROPFILES>();
    assert_eq!(header, 20);
    let field = |i: usize| u32::from_ne_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    assert_eq!((field(0), field(1), field(2), field(3), field(4)), (header as u32, 0, 0, 0, 1));
    let units: Vec<u16> = bytes[header..].chunks(2).map(|c| u16::from_ne_bytes([c[0], c[1]])).collect();
    let expected: Vec<u16> = r"C:\a".encode_utf16().chain([0]).chain(r"D:\bé".encode_utf16()).chain([0, 0]).collect();
    assert_eq!(units, expected);
}
