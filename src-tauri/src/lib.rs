// Library root so the app builds as both a desktop binary and a Tauri library.
mod ant_core;
mod commands;
mod config;
mod error;
mod fs_safe;
mod index;
mod mutations;
mod storage;
#[cfg(test)]
mod test_support;
mod tree;
mod updater;

use commands::settings;

pub fn run() {
    let start = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let storage = commands::Storage::resolve(start);
    let title = storage.root.as_ref().and_then(|r| r.file_name()).map(|n| n.to_string_lossy().into_owned());

    tauri::Builder::default()
        .manage(storage)
        .setup(move |app| {
            use tauri::Manager;
            if let (Some(name), Some(window)) = (&title, app.get_webview_window("main")) {
                window.set_title(&format!("ANT System Manager v.{} ({name})", updater::CURRENT_VERSION))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_storage,
            commands::load_tree,
            commands::list_children,
            commands::open_node,
            commands::refresh,
            commands::rename_node,
            commands::set_node_type,
            commands::add_node,
            commands::move_node,
            commands::inspect_delete,
            commands::delete_node,
            commands::skip_version,
            commands::set_node_details,
            commands::set_favorite,
            settings::get_settings,
            settings::set_setting,
            settings::open_link,
            commands::check_update,
            commands::start_update,
            commands::search_nodes,
            commands::suggest_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Folder Manager");
}
