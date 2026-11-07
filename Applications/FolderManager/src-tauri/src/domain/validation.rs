// Checks on user-entered values that hold without touching the disk.
const MAX_BACKUP_FOLDER_CHARS: usize = 400;

// Empty means the default folder inside the storage.
pub(crate) fn valid_backup_folder(value: &str) -> bool {
    let path = std::path::Path::new(value);
    value.is_empty()
        || (value.chars().count() <= MAX_BACKUP_FOLDER_CHARS
            && !value.chars().any(char::is_control)
            && path.is_absolute()
            && !path.components().any(|c| c == std::path::Component::ParentDir))
}

// Every prefix but a drive letter can reach another host, verbatim and device forms included.
pub(crate) fn remote_backup_folder(value: &str) -> bool {
    use std::path::{Component, Path, Prefix};
    match Path::new(value).components().next() {
        Some(Component::Prefix(prefix)) => !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)),
        _ => false,
    }
}

pub(crate) fn valid_stamp(stamp: &str) -> bool {
    let groups: Vec<&str> = stamp.split('-').collect();
    groups.len() == 5 && groups.iter().all(|g| g.len() == 2 && g.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_five_two_digit_groups_make_a_stamp() {
        assert!(valid_stamp("26-10-24-09-05"));
        assert!(!valid_stamp("26-10-24-09"));
        assert!(!valid_stamp("26-10-24-09-5"));
        assert!(!valid_stamp("26-10-24-09-0a"));
        assert!(!valid_stamp("../x-10-24-09-05"));
        assert!(!valid_stamp("26-10-24-09-05-00"));
    }

    #[cfg(windows)]
    #[test]
    fn only_a_plain_drive_path_counts_as_local() {
        for remote in [r"\\host\share", "//host/share", r"\\?\UNC\host\share", r"\\.\UNC\host\share", r"\\?\GLOBALROOT\Device\Mup\host\share"] {
            assert!(remote_backup_folder(remote), "{remote}");
        }
        for local in ["", r"C:\Backups", "D:/Backups", r"\\?\C:\Backups"] {
            assert!(!remote_backup_folder(local), "{local}");
        }
    }
}
