// Library root so the app builds as both a desktop binary and a Tauri library.
mod ant_core;
mod backup;
mod commands;
mod config;
mod debug_log;
mod error;
mod extensions;
mod fs_safe;
mod index;
mod mutations;
mod scheduler;
mod storage;
mod sync;
#[cfg(test)]
mod test_support;
mod tree;
mod updater;

use commands::settings;

pub fn run() {
    if std::env::args_os().nth(1).is_some_and(|a| a == "--backup") {
        std::process::exit(backup::run_headless(std::env::args_os().nth(2)));
    }
    if std::env::args_os().nth(1).is_some_and(|a| a == "--sync") {
        std::process::exit(sync::run_headless(std::env::args_os().nth(2)));
    }
    let start = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let storage = commands::Storage::resolve(start);
    let title = storage.root().ok().and_then(|r| r.file_name().map(|n| n.to_string_lossy().into_owned()));

    tauri::Builder::default()
        .manage(storage)
        .register_uri_scheme_protocol(extensions::sandbox_page::SCHEME, |_ctx, _request| extensions::sandbox_page::response())
        .setup(move |app| {
            use tauri::Manager;
            if let (Some(name), Some(window)) = (&title, app.get_webview_window("main")) {
                window.set_title(&commands::window_title(name))?;
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
            commands::start_debug_log,
            commands::stop_debug_log,
            commands::log_debug,
            commands::open_logs_folder,
            commands::open_devtools,
            settings::reset_settings,
            commands::backup_now,
            commands::backup_if_due,
            commands::list_backups,
            commands::restore_backup,
            commands::backup_schedule,
            commands::rename_storage,
            commands::revert_storage_rename,
            commands::storage_rename_status,
            commands::sync_schedule,
            commands::list_extensions,
            commands::set_extension_enabled,
            commands::read_extension_entry,
            commands::remove_extension,
            commands::list_registry,
            commands::install_registry_extension,
            commands::inspect_extension_repo,
            commands::install_inspected_extension,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Folder Manager");
}
