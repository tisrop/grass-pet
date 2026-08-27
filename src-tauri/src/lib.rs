mod commands;
mod context_menu;
mod geometry;
mod reminders;
mod state;
mod tray;
mod windows;

use chrono::Local;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_notification::init());
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_wdio::init());

    let app = builder
        .setup(|app| {
            #[cfg(feature = "e2e")]
            let data_path = std::env::var_os("GRASS_PET_E2E_STATE")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("state-e2e.json"));
            #[cfg(not(feature = "e2e"))]
            let data_path = app.path().app_data_dir()?.join("state.json");
            let state = AppState::load(data_path);
            state.normalize_stats_day(&Local::now().date_naive().to_string())?;
            let settings = state
                .data
                .lock()
                .map_err(|_| "state lock is poisoned")?
                .settings
                .clone();
            app.manage(state);
            commands::apply_pet_settings(app.handle(), &settings)?;
            windows::install_close_to_hide(app.handle());
            windows::place_pet_near_bottom_right(app.handle());
            tray::install(app)?;
            windows::spawn_auto_walk(app.handle().clone());
            windows::spawn_companion_tracker(app.handle().clone());
            reminders::spawn_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_get,
            commands::settings_update,
            commands::reminders_list,
            commands::reminders_save,
            commands::reminders_remove,
            commands::interactions_list,
            commands::interactions_stats,
            commands::interactions_trigger,
            commands::window_start_dragging,
            commands::window_finish_drag,
            commands::window_show_context_menu,
            commands::window_hide_context_menu,
            commands::window_show_dashboard,
            commands::window_show_reminder,
            commands::window_hide,
            commands::walk_set_paused,
            commands::runtime_ready,
            commands::runtime_fail,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the Tauri application");
    app.run(|handle, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            let _ = handle.state::<AppState>().persist();
        }
    });
}
