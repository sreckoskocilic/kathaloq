mod commands;
pub mod db;
pub mod models;
pub mod scanner;

use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let database = Database::new(app_dir).expect("failed to initialize database");
            app.manage(database);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_scan,
            commands::list_catalogs,
            commands::delete_catalog,
            commands::get_children,
            commands::search_files,
            commands::get_children_filtered,
            commands::preview_catalog_update,
            commands::apply_catalog_update,
            commands::remove_file_entries,
            commands::get_folder_stats,
            commands::get_bulk_stats,
            commands::get_media_tags,
            commands::get_media_tags_bulk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
