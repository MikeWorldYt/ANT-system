// Where the storage template and tools are downloaded from, and the folders it fills.
pub const TEMPLATE_DB_URL: &str = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/Core/ant.db";
// Retired: storages are built from the template database, so this file is never written.
pub const RETIRED_TREE_NAME: &str = "Tree.yaml";
pub const CORE_API_URL: &str = "https://api.github.com/repos/MikeWorldYt/ANT-system/contents/Migration/Core";
// FolderManager comes from the GitHub releases tagged `folder-manager-vX.Y.Z`; the newest stable one is used.
pub const RELEASES_API_URL: &str = "https://api.github.com/repos/MikeWorldYt/ANT-system/releases?per_page=100";
pub const RELEASE_DOWNLOAD_URL: &str = "https://github.com/MikeWorldYt/ANT-system/releases/download";
pub const FOLDER_MANAGER_TAG_PREFIX: &str = "folder-manager-v";
// Used when the releases list cannot be read (offline API, rate limit).
pub const FOLDER_MANAGER_FALLBACK_TAG: &str = "folder-manager-v2.4.0";
// The release asset is saved under this same name in the storage root.
#[cfg(windows)]
pub const FOLDER_MANAGER_NAME: &str = "FolderManager.exe";
#[cfg(not(windows))]
pub const FOLDER_MANAGER_NAME: &str = "FolderManager";
pub const REVIEW_DIR: &str = "998.Review";
pub const SETTINGS_DIR: &str = "999.Settings";
pub const CORE_DIR: &str = "00.Core";
pub const USER_AGENT: &str = "User-Agent: ANT-Setup-Wizard";
pub const ANT_DIR: &str = ".ant";
pub const DB_NAME: &str = "ant.db";
pub const INFO_NAME: &str = "info.json";
