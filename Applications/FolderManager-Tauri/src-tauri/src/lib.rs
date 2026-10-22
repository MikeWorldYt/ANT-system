// Library root so the app builds as both a desktop binary and a Tauri library.
mod ant_core;
mod commands;
mod config;
mod edit_actions;
mod error;
mod fs_safe;
mod move_delete;
mod names;
mod review_index;
mod tree_node;
mod yaml_store;

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
                window.set_title(&format!("ANT System - Folder Manager - {name}"))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_storage,
            commands::load_tree,
            commands::list_children,
            commands::refresh,
            commands::rename_node,
            commands::set_node_type,
            commands::add_child,
            commands::move_node,
            commands::inspect_delete,
            commands::delete_node,
            commands::get_skipped_version,
            commands::skip_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Folder Manager");
}
