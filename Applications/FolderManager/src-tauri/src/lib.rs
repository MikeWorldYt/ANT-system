// Library root so the app builds as both a desktop binary and a Tauri library.
mod app;
mod commands;
mod domain;
mod infra;
#[cfg(test)]
mod ipc_parity_tests;
#[cfg(test)]
mod layer_tests;
mod platform;
mod services;
#[cfg(test)]
mod test_support;

use app::storage_session::Storage;
use app::window_state;
use commands::settings;

const STORAGE_CHANGED: &str = "storage-changed";

pub fn run() {
    if std::env::args_os().nth(1).is_some_and(|a| a == "--backup") {
        std::process::exit(services::backup::run_headless(std::env::args_os().nth(2)));
    }
    if std::env::args_os().nth(1).is_some_and(|a| a == "--sync") {
        std::process::exit(services::sync::run_headless(std::env::args_os().nth(2)));
    }
    if std::env::args_os().nth(1).is_some_and(|a| a == "--watch") {
        std::process::exit(services::watch::run_headless(std::env::args_os().nth(2)));
    }
    let arg = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let cwd = std::env::current_dir().unwrap_or_default();
    let storage = Storage::resolve(cwd.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(storage)
        .register_uri_scheme_protocol(services::extensions::sandbox_page::SCHEME, |_ctx, _request| services::extensions::sandbox_page::response())
        .setup(move |app| {
            use tauri::Manager;
            let remembered = infra::storage::pointer::app_file(app.handle()).ok().and_then(|f| infra::storage::pointer::read(&f));
            let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(std::path::Path::to_path_buf));
            let state = app.state::<Storage>();
            state.reset(infra::storage::location::launch_start(arg, remembered, cwd, exe_dir));
            if let Ok(root) = state.root() {
                infra::storage::pointer::remember(app.handle(), &root);
                if let Some(window) = app.get_webview_window("main") {
                    window.set_title(&commands::window_title(&infra::storage::rename::folder_name(&root)))?;
                }
            }
            let handle = app.handle().clone();
            let emitter = app.handle().clone();
            app.manage(services::watch::GuiWatch::start(
                move || handle.state::<Storage>().root().ok(),
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
            commands::reveal_node,
            commands::refresh,
            commands::check_root,
            commands::rename_node,
            commands::set_node_type,
            commands::add_node,
            commands::add_child,
            commands::move_node,
            commands::move_folder,
            commands::preview_invalid,
            commands::resolve_invalid,
            commands::inspect_delete,
            commands::delete_node,
            commands::skip_version,
            commands::set_node_details,
            commands::set_node_output,
            commands::set_outcome_default,
            commands::deliver_output,
            commands::restore_output,
            commands::set_node_hidden,
            commands::set_hidden_nodes,
            commands::list_hidden_choices,
            commands::set_favorite,
            commands::set_important,
            commands::set_link_hidden,
            commands::file_image,
            commands::list_downloads,
            commands::open_download,
            commands::copy_downloads,
            commands::download_image,
            commands::rename_file,
            commands::delete_file,
            commands::copy_items,
            commands::move_files_in,
            commands::create_file_shortcut_in,
            commands::set_review_filter,
            commands::answer_copy,
            commands::set_user_flag,
            settings::get_settings,
            settings::set_setting,
            settings::open_link,
            commands::check_update,
            commands::start_update,
            commands::search_nodes,
            commands::list_favorites,
            commands::list_review,
            commands::list_workboards,
            commands::set_board,
            commands::search_disk,
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
            commands::acknowledge_intake,
            commands::check_support_upgrade,
            commands::apply_support_upgrade,
            commands::finish_support_upgrade,
            commands::drop_files,
            commands::paste_files,
            commands::start_drag_out,
            commands::create_shortcut_in,
        ])
        .build(tauri::generate_context!())
        .expect("error while running Folder Manager")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(watcher) = app.try_state::<services::watch::GuiWatch>() {
                    watcher.shutdown();
                }
            }
        });
}
