use crate::discord_presence::PresenceClient;
use crate::{
    app_config::{
        DEFAULT_ACTIVITY_TYPE, DEFAULT_BUTTON_1_LABEL, DEFAULT_BUTTON_1_URL,
        DEFAULT_BUTTON_2_LABEL, DEFAULT_BUTTON_2_URL, DEFAULT_CUSTOM_TIMESTAMP_END,
        DEFAULT_CUSTOM_TIMESTAMP_START, DEFAULT_ICON_KEY, DEFAULT_ICON_TEXT, DEFAULT_IDLE_MESSAGE,
        DEFAULT_PRESENCE_MESSAGE, DEFAULT_RPC_NAME, DEFAULT_SHARE_BUTTON_LABEL,
        DEFAULT_SMALL_ICON_KEY, DEFAULT_SMALL_ICON_TEXT, DEFAULT_STATE_TEXT,
        DEFAULT_TIMESTAMP_MODE, DISCORD_CLIENT_ID,
    },
    clip_studio::{detect_clip_studio, ClipStudioDetection},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default = "default_discord_client_id")]
    pub discord_client_id: String,
    #[serde(default = "default_activity_type")]
    pub activity_type: String,
    #[serde(default = "default_rpc_name")]
    pub rpc_name: String,
    #[serde(default)]
    pub rpc_name_from_document: bool,
    #[serde(default = "default_presence_message")]
    pub presence_message: String,
    #[serde(default = "default_idle_message")]
    pub idle_message: String,
    #[serde(default = "default_state_text")]
    pub state_text: String,
    #[serde(default = "default_icon_key")]
    pub icon_key: String,
    #[serde(default = "default_icon_text")]
    pub icon_text: String,
    #[serde(default = "default_small_icon_key")]
    pub small_icon_key: String,
    #[serde(default = "default_small_icon_text")]
    pub small_icon_text: String,
    #[serde(default = "default_button_1_label")]
    pub button_1_label: String,
    #[serde(default = "default_button_1_url")]
    pub button_1_url: String,
    #[serde(default = "default_button_2_label")]
    pub button_2_label: String,
    #[serde(default = "default_button_2_url")]
    pub button_2_url: String,
    #[serde(default = "default_share_button_label")]
    pub share_button_label: String,
    #[serde(default = "default_timestamp_mode")]
    pub timestamp_mode: String,
    #[serde(default = "default_custom_timestamp_start")]
    pub custom_timestamp_start: i64,
    #[serde(default = "default_custom_timestamp_end")]
    pub custom_timestamp_end: i64,
    #[serde(default)]
    pub party_size: u32,
    #[serde(default)]
    pub party_max: u32,
    #[serde(default = "default_true")]
    pub show_document_name: bool,
    #[serde(default = "default_true")]
    pub show_elapsed_time: bool,
    #[serde(default = "default_true")]
    pub show_procrastination_percent: bool,
    #[serde(default)]
    pub only_when_focused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            discord_client_id: default_discord_client_id(),
            activity_type: default_activity_type(),
            rpc_name: default_rpc_name(),
            rpc_name_from_document: false,
            presence_message: default_presence_message(),
            idle_message: default_idle_message(),
            state_text: default_state_text(),
            icon_key: default_icon_key(),
            icon_text: default_icon_text(),
            small_icon_key: default_small_icon_key(),
            small_icon_text: default_small_icon_text(),
            button_1_label: default_button_1_label(),
            button_1_url: default_button_1_url(),
            button_2_label: default_button_2_label(),
            button_2_url: default_button_2_url(),
            share_button_label: default_share_button_label(),
            timestamp_mode: default_timestamp_mode(),
            custom_timestamp_start: default_custom_timestamp_start(),
            custom_timestamp_end: default_custom_timestamp_end(),
            party_size: 0,
            party_max: 0,
            show_document_name: true,
            show_elapsed_time: true,
            show_procrastination_percent: true,
            only_when_focused: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AppStatus {
    pub settings: Settings,
    pub clip_studio_running: bool,
    pub clip_studio_focused: bool,
    pub document_title: Option<String>,
    pub shared_screenshot_url: Option<String>,
    pub discord_connected: bool,
    pub discord_error: Option<String>,
    pub procrastination_percent: Option<u8>,
    pub last_updated_unix: u64,
}

#[derive(Clone, Debug)]
struct FocusStats {
    focused_seconds: u64,
    idle_seconds: u64,
    last_sample_unix: u64,
}

#[derive(Clone, Debug)]
struct RuntimeState {
    settings: Settings,
    detection: ClipStudioDetection,
    shared_screenshot_url: Option<String>,
    discord_connected: bool,
    discord_error: Option<String>,
    focus_stats: FocusStats,
    last_document_title: Option<String>,
    last_updated_unix: u64,
}

pub struct AppState {
    inner: Arc<Mutex<RuntimeState>>,
    config_path: PathBuf,
}

impl AppState {
    pub fn load(app: AppHandle) -> Self {
        let config_path = config_path(&app);
        let settings = fs::read_to_string(&config_path)
            .ok()
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .unwrap_or_default();
        let loaded_at = now_unix();

        Self {
            inner: Arc::new(Mutex::new(RuntimeState {
                settings,
                detection: ClipStudioDetection::default(),
                shared_screenshot_url: None,
                discord_connected: false,
                discord_error: None,
                focus_stats: FocusStats {
                    focused_seconds: 0,
                    idle_seconds: 0,
                    last_sample_unix: loaded_at,
                },
                last_document_title: None,
                last_updated_unix: loaded_at,
            })),
            config_path,
        }
    }

    pub fn snapshot(&self) -> AppStatus {
        let inner = self.inner.lock().expect("app state lock poisoned");

        AppStatus {
            settings: inner.settings.clone(),
            clip_studio_running: inner.detection.running,
            clip_studio_focused: inner.detection.focused,
            document_title: inner.detection.document_title.clone(),
            shared_screenshot_url: inner.shared_screenshot_url.clone(),
            discord_connected: inner.discord_connected,
            discord_error: inner.discord_error.clone(),
            procrastination_percent: inner.focus_stats.procrastination_percent(),
            last_updated_unix: inner.last_updated_unix,
        }
    }

    pub fn save_settings(&self, settings: Settings) -> Result<(), SaveSettingsError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&self.config_path, json)?;

        let mut inner = self.inner.lock().expect("app state lock poisoned");
        inner.settings = settings;
        Ok(())
    }

    pub fn set_shared_screenshot(&self, url: String) {
        let mut inner = self.inner.lock().expect("app state lock poisoned");
        inner.shared_screenshot_url = Some(url);
        inner.last_updated_unix = now_unix();
    }

    pub fn spawn_monitor(&self) {
        let state = self.clone_for_thread();

        thread::spawn(move || {
            let mut presence = PresenceClient::default();

            loop {
                let (settings, shared_screenshot_url) = {
                    let inner = state.inner.lock().expect("app state lock poisoned");
                    (inner.settings.clone(), inner.shared_screenshot_url.clone())
                };

                let mut detection = detect_clip_studio();
                let procrastination_percent = {
                    let mut inner = state.inner.lock().expect("app state lock poisoned");
                    if !detection.running {
                        inner.last_document_title = None;
                    } else if let Some(title) = detection.document_title.clone() {
                        inner.last_document_title = Some(title);
                    } else {
                        detection.document_title = inner.last_document_title.clone();
                    }
                    inner.focus_stats.update(&detection);
                    inner.focus_stats.procrastination_percent()
                };
                let presence_state = presence.sync(
                    &settings,
                    &detection,
                    shared_screenshot_url.as_deref(),
                    procrastination_percent,
                );

                {
                    let mut inner = state.inner.lock().expect("app state lock poisoned");
                    inner.detection = detection;
                    inner.discord_connected = presence_state.connected;
                    inner.discord_error = presence_state.error;
                    inner.last_updated_unix = now_unix();
                }

                thread::sleep(Duration::from_secs(3));
            }
        });
    }

    fn clone_for_thread(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            config_path: self.config_path.clone(),
        }
    }
}

impl FocusStats {
    fn update(&mut self, detection: &ClipStudioDetection) {
        let now = now_unix();
        let elapsed = now.saturating_sub(self.last_sample_unix);
        self.last_sample_unix = now;

        if !detection.running || elapsed == 0 {
            return;
        }

        if detection.focused {
            self.focused_seconds = self.focused_seconds.saturating_add(elapsed);
        } else {
            self.idle_seconds = self.idle_seconds.saturating_add(elapsed);
        }
    }

    fn procrastination_percent(&self) -> Option<u8> {
        let total = self.focused_seconds.saturating_add(self.idle_seconds);
        if total == 0 {
            return None;
        }

        Some(((self.idle_seconds.saturating_mul(100) + total / 2) / total).min(100) as u8)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SaveSettingsError {
    #[error("could not write settings file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not serialize settings: {0}")]
    Json(#[from] serde_json::Error),
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("settings.json")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn default_discord_client_id() -> String {
    DISCORD_CLIENT_ID.to_string()
}

fn default_presence_message() -> String {
    DEFAULT_PRESENCE_MESSAGE.to_string()
}

fn default_rpc_name() -> String {
    DEFAULT_RPC_NAME.to_string()
}

fn default_idle_message() -> String {
    DEFAULT_IDLE_MESSAGE.to_string()
}

fn default_activity_type() -> String {
    DEFAULT_ACTIVITY_TYPE.to_string()
}

fn default_icon_key() -> String {
    DEFAULT_ICON_KEY.to_string()
}

fn default_state_text() -> String {
    DEFAULT_STATE_TEXT.to_string()
}

fn default_icon_text() -> String {
    DEFAULT_ICON_TEXT.to_string()
}

fn default_small_icon_key() -> String {
    DEFAULT_SMALL_ICON_KEY.to_string()
}

fn default_small_icon_text() -> String {
    DEFAULT_SMALL_ICON_TEXT.to_string()
}

fn default_button_1_label() -> String {
    DEFAULT_BUTTON_1_LABEL.to_string()
}

fn default_button_1_url() -> String {
    DEFAULT_BUTTON_1_URL.to_string()
}

fn default_button_2_label() -> String {
    DEFAULT_BUTTON_2_LABEL.to_string()
}

fn default_button_2_url() -> String {
    DEFAULT_BUTTON_2_URL.to_string()
}

fn default_share_button_label() -> String {
    DEFAULT_SHARE_BUTTON_LABEL.to_string()
}

fn default_timestamp_mode() -> String {
    DEFAULT_TIMESTAMP_MODE.to_string()
}

fn default_custom_timestamp_start() -> i64 {
    DEFAULT_CUSTOM_TIMESTAMP_START
}

fn default_custom_timestamp_end() -> i64 {
    DEFAULT_CUSTOM_TIMESTAMP_END
}

fn default_true() -> bool {
    true
}
