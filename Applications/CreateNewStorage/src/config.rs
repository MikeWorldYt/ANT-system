// Where the storage template and tools are downloaded from, and the folders it fills.
pub const TEMPLATE_DB_URL: &str = "https://r2.mikeworld.dev/core/ant.db";
pub const CORE_BASE_URL: &str = "https://r2.mikeworld.dev/core";
pub const CORE_FILES: [&str; 1] = ["FM_Updater.bat"];
// FolderManager comes from the GitHub releases tagged `folder-manager-vX.Y.Z`; the newest stable one is used.
pub const RELEASES_API_URL: &str = "https://api.github.com/repos/MikeWorldYt/ANT-system/releases?per_page=100";
pub const RELEASE_DOWNLOAD_URL: &str = "https://github.com/MikeWorldYt/ANT-system/releases/download";
pub const FOLDER_MANAGER_TAG_PREFIX: &str = "folder-manager-v";
// Used when the releases list cannot be read (offline API, rate limit).
pub const FOLDER_MANAGER_FALLBACK_TAG: &str = "folder-manager-v2.4.0";
#[cfg(windows)]
pub const FOLDER_MANAGER_NAME: &str = "FolderManager.exe";
#[cfg(not(windows))]
pub const FOLDER_MANAGER_NAME: &str = "FolderManager";
#[cfg(windows)]
pub const FOLDER_MANAGER_ASSET: &str = "FolderManager.exe";
#[cfg(target_os = "macos")]
pub const FOLDER_MANAGER_ASSET: &str = "FolderManager-macos.tar.gz";
#[cfg(not(any(windows, target_os = "macos")))]
pub const FOLDER_MANAGER_ASSET: &str = "FolderManager-linux.tar.gz";
// The binary's name inside the Linux and macOS release archives.
#[cfg(not(windows))]
pub const FOLDER_MANAGER_MEMBER: &str = "folder-manager";
pub const REVIEW_DIR: &str = "998.Review";
pub const SETTINGS_DIR: &str = "999.Settings";
pub const CORE_DIR: &str = "00.Core";
pub const USER_AGENT: &str = "User-Agent: ANT-Setup-Wizard";
pub const ANT_DIR: &str = ".ant";
pub const DB_NAME: &str = "ant.db";
pub const INFO_NAME: &str = "info.json";
