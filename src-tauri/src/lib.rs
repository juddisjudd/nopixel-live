mod commands;
mod kick;
mod merge;
mod model;
mod player;
mod poller;
mod settings;
mod tray;
mod twitch;

use tauri::{Manager, WindowEvent};

use crate::poller::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState::new(settings_path)?);
            tray::create(app.handle())?;
            tauri::async_runtime::spawn(poller::run(app.handle().clone()));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.state::<AppState>().settings().close_to_tray
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_settings,
            commands::save_settings,
            commands::toggle_favorite,
            commands::hide_streamer,
            commands::unhide_streamer,
            commands::refresh_now,
            commands::play,
            commands::open_page,
            commands::detect_tools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NoPixel Live");
}
