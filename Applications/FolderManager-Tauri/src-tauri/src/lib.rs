// Library root so the app builds as both a desktop binary and a Tauri library.
mod ant_core;
mod config;
mod error;
mod fs_safe;
mod names;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Folder Manager");
}
