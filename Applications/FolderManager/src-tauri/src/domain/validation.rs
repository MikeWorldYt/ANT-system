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
