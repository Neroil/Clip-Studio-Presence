mod app_config;
mod app_state;
mod capture_share;
mod clip_studio;
mod discord_presence;

use app_state::{AppState, AppStatus, Settings};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> AppStatus {
    state.snapshot()
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<AppStatus, String> {
    state
        .save_settings(settings)
        .map_err(|error| error.to_string())?;
    Ok(state.snapshot())
}

#[tauri::command]
fn capture_and_share(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppStatus, String> {
    let result = capture_share::capture_and_upload(&app).map_err(|error| error.to_string())?;
    state.set_shared_screenshot(result.url);
    Ok(state.snapshot())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::load(app.handle().clone());
            state.spawn_monitor();
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            save_settings,
            capture_and_share
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Clip Studio Presence");
}
