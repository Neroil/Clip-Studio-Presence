mod app_config;
mod app_state;
mod capture_share;
mod clip_studio;
mod discord_presence;
mod startup;
mod updates;

use app_state::{AppState, AppStatus, Settings};
use tauri::{AppHandle, Manager, State, WindowEvent};

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> AppStatus {
    state.snapshot()
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<AppStatus, String> {
    startup::set_start_on_boot(settings.start_on_boot).map_err(|error| error.to_string())?;
    state
        .save_settings(settings)
        .map_err(|error| error.to_string())?;
    Ok(state.snapshot())
}

#[tauri::command]
fn capture_and_share(app: AppHandle, state: State<'_, AppState>) -> Result<AppStatus, String> {
    let settings = state.snapshot().settings;
    let result =
        capture_share::capture_and_upload(&app, &settings).map_err(|error| error.to_string())?;
    state.set_shared_screenshot(result.url);
    Ok(state.snapshot())
}

#[tauri::command]
fn check_for_updates() -> Result<updates::UpdateCheckResult, String> {
    updates::check_for_updates().map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::load(app.handle().clone());
            if let Err(error) = startup::set_start_on_boot(state.snapshot().settings.start_on_boot)
            {
                eprintln!("could not sync start-on-boot setting: {error}");
            }
            state.spawn_monitor(app.handle().clone());
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.minimize() {
                    eprintln!("could not minimize window on close: {error}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            save_settings,
            capture_and_share,
            check_for_updates
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Clip Studio Presence");
}
