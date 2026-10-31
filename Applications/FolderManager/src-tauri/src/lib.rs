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
mod intake;
mod mutations;
mod scheduler;
mod storage;
mod sync;
#[cfg(test)]
mod test_support;
mod tree;
mod updater;
mod watch;
mod window_state;

use commands::settings;

const STORAGE_CHANGED: &str = "storage-changed";

pub fn run() {
    if std::env::args_os().nth(1).is_some_and(|a| a == "--backup") {
        std::process::exit(backup::run_headless(std::env::args_os().nth(2)));
    }
    if std::env::args_os().nth(1).is_some_and(|a| a == "--sync") {
        std::process::exit(sync::run_headless(std::env::args_os().nth(2)));
    }
    if std::env::args_os().nth(1).is_some_and(|a| a == "--watch") {
        std::process::exit(watch::run_headless(std::env::args_os().nth(2)));
    }
    let arg = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let cwd = std::env::current_dir().unwrap_or_default();
    let storage = commands::Storage::resolve(cwd.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(storage)
        .register_uri_scheme_protocol(extensions::sandbox_page::SCHEME, |_ctx, _request| extensions::sandbox_page::response())
        .setup(move |app| {
            use tauri::Manager;
            let remembered = storage::pointer::app_file(app.handle()).ok().and_then(|f| storage::pointer::read(&f));
            let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(std::path::Path::to_path_buf));
            let state = app.state::<commands::Storage>();
            state.reset(storage::location::launch_start(arg, remembered, cwd, exe_dir));
            if let Ok(root) = state.root() {
                storage::pointer::remember(app.handle(), &root);
                if let Some(window) = app.get_webview_window("main") {
                    window.set_title(&commands::window_title(&storage::rename::folder_name(&root)))?;
                }
            }
            let handle = app.handle().clone();
            let emitter = app.handle().clone();
            app.manage(watch::GuiWatch::start(
                move || handle.state::<commands::Storage>().root().ok(),
                move |event| {
                    use tauri::Emitter;
                    if let Err(e) = emitter.emit(STORAGE_CHANGED, event) {
                        eprintln!("Could not send {STORAGE_CHANGED}: {e}");
                    }
                },
            ));
            if let Some(window) = app.get_webview_window("main") {
                window_state::restore(&window);
                let saved = window.clone();
                window.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                        window_state::save(&saved);
                    }
                });
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
            commands::add_child,
            commands::move_node,
            commands::move_folder,
            commands::inspect_delete,
            commands::delete_node,
            commands::skip_version,
            commands::set_node_details,
            commands::set_favorite,
            commands::answer_copy,
            commands::set_user_flag,
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
            commands::storage_health,
            commands::use_latest_backup,
            commands::prepare_scan,
            commands::change_storage_path,
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
            commands::run_intake,
            commands::drop_files,
            commands::paste_files,
            commands::copy_folder,
            commands::start_drag_out,
            commands::create_shortcut_in,
        ])
        .build(tauri::generate_context!())
        .expect("error while running Folder Manager")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(watcher) = app.try_state::<watch::GuiWatch>() {
                    watcher.shutdown();
                }
            }
        });
}
