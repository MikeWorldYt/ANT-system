// Where the storage template is downloaded from, and the hidden folder it fills.
pub const TEMPLATE_URL: &str = "https://r2.mikeworld.dev/core/template.md";
// FolderManager is the portable asset named in the release channel's latest.json, checked against its SHA-256.
pub const LATEST_JSON_URL: &str = "https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-latest/latest.json";
pub const RELEASES_PAGE_URL: &str = "https://github.com/MikeWorldYt/ANT-system/releases";
#[cfg(windows)]
pub const FOLDER_MANAGER_NAME: &str = "FolderManager.exe";
#[cfg(not(windows))]
pub const FOLDER_MANAGER_NAME: &str = "FolderManager";
#[cfg(windows)]
pub const FOLDER_MANAGER_ASSET: &str = "FolderManager_portable.exe";
#[cfg(target_os = "macos")]
pub const FOLDER_MANAGER_ASSET: &str = "FolderManager-macos_portable.tar.gz";
#[cfg(not(any(windows, target_os = "macos")))]
pub const FOLDER_MANAGER_ASSET: &str = "FolderManager-linux_portable.tar.gz";
pub const USER_AGENT: &str = "User-Agent: ANT-Setup-Wizard";
pub const ANT_DIR: &str = ".ant";
pub const TEMPLATE_NAME: &str = "template.md";
pub const INFO_NAME: &str = "info.json";
