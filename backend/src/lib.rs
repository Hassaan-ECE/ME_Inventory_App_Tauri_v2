pub(crate) mod api;
pub(crate) mod domain;
pub(crate) mod integrations;
pub(crate) mod runtime;
pub(crate) mod storage;
mod sync;

pub(crate) use api::commands;
pub(crate) use domain::{model, query};
pub(crate) use integrations::{deprecated_db_cleanup, export, native};
pub(crate) use runtime::shared_watcher;
pub(crate) use storage as store;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let db = store::InventoryDb::open(app.handle())?;
            let _ = deprecated_db_cleanup::quarantine_deprecated_databases_once(app.handle(), &db);
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
