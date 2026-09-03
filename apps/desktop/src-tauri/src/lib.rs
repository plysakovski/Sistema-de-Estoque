mod auth;
mod commands;
mod database;
mod encryption;
mod error;
mod imports;
mod models;

use auth::AuthService;
use database::Database;
use imports::ImportService;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(5_000_000)
                .build(),
        )
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            let database = Database::open(data_directory.join("stockmanager.db"))?;
            log::info!("Banco inicializado em {}", database.path().display());
            app.manage(database);
            app.manage(AuthService::new()?);
            app.manage(ImportService::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth_status,
            commands::bootstrap_admin,
            commands::login,
            commands::logout,
            commands::get_dashboard,
            commands::list_items,
            commands::create_item,
            commands::list_locations,
            commands::list_location_records,
            commands::save_location,
            commands::set_location_active,
            commands::list_categories,
            commands::save_category,
            commands::set_category_active,
            commands::list_assets,
            commands::list_asset_records,
            commands::update_asset,
            commands::list_audit_logs,
            commands::list_movements,
            commands::create_movement,
            commands::create_movement_batch,
            commands::create_asset_incident,
            commands::list_asset_incidents,
            commands::review_asset_incident,
            commands::create_backup,
            commands::list_backups,
            commands::restore_backup,
            commands::recovery_key_status,
            commands::reveal_recovery_key,
            commands::acknowledge_recovery_key,
            commands::preview_import,
            commands::confirm_import,
            commands::discard_import,
            commands::create_adjustment_request,
            commands::list_adjustment_requests,
            commands::review_adjustment_request,
            commands::create_replenishment_request,
            commands::list_replenishment_requests,
            commands::review_replenishment_request,
            commands::dispatch_replenishment_transfer,
            commands::receive_replenishment_transfer,
            commands::list_users,
            commands::create_user,
            commands::update_user,
            commands::reset_user_password,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar StockManager Pro");
}
