mod commands;
mod export;
mod legacy_import;
mod model;
mod native;
mod query;
mod shared_watcher;
mod store;
mod sync;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let legacy_sqlite_path = legacy_import::resolve_legacy_sqlite_path(app.handle());
            let db = store::InventoryDb::open(app.handle(), legacy_sqlite_path)?;
            app.manage(db);
            app.manage(shared_watcher::SharedSyncWatcher::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_inventory,
            commands::query_inventory,
            commands::sync_inventory,
            commands::create_entry,
            commands::update_entry,
            commands::toggle_verified_entry,
            commands::set_archived_entry,
            commands::delete_entry,
            commands::import_legacy_sqlite,
            export::export_excel,
            native::load_picture_preview,
            native::open_external,
            native::open_path,
            native::pick_picture_path
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            app_handle.state::<store::InventoryDb>().flush();
        }
    });
}
