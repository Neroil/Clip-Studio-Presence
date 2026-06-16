use crate::{
    app_config::{
        DEFAULT_ACTIVITY_TYPE, DEFAULT_ICON_KEY, DEFAULT_ICON_TEXT, DEFAULT_IDLE_MESSAGE,
        DEFAULT_PRESENCE_MESSAGE, DEFAULT_RPC_NAME, DEFAULT_SHARE_BUTTON_LABEL, DEFAULT_STATE_TEXT,
        DISCORD_CLIENT_ID,
    },
    app_state::Settings,
    clip_studio::ClipStudioDetection,
};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde_json::{json, Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct PresenceState {
    pub connected: bool,
    pub error: Option<String>,
}

pub struct PresenceClient {
    client: Option<DiscordIpcClient>,
    client_id: String,
    active_since: Option<i64>,
    app_started_at: i64,
}

impl Default for PresenceClient {
    fn default() -> Self {
        Self {
            client: None,
            client_id: String::new(),
            active_since: None,
            app_started_at: now_unix(),
        }
    }
}

impl PresenceClient {
    pub fn sync(
        &mut self,
        settings: &Settings,
        detection: &ClipStudioDetection,
        shared_screenshot_url: Option<&str>,
        procrastination_percent: Option<u8>,
    ) -> PresenceState {
        if !detection.running {
            self.active_since = None;
            self.clear_activity();
            return PresenceState::default();
        }

        let client_id = configured_client_id(settings);
        if self.client_id != client_id || self.client.is_none() {
            self.disconnect();
            self.client_id = client_id;

            match DiscordIpcClient::new(self.client_id.as_str()).and_then(|mut client| {
                client.connect()?;
                Ok(client)
            }) {
                Ok(client) => self.client = Some(client),
                Err(error) => {
                    return PresenceState {
                        connected: false,
                        error: Some(format!("Could not connect to Discord: {error}")),
                    };
                }
            }
        }

        if self.active_since.is_none() {
            self.active_since = Some(now_unix());
        }

        let activity = self.activity(
            settings,
            detection,
            shared_screenshot_url,
            procrastination_percent,
        );

        match self
            .client
            .as_mut()
            .expect("presence client should be connected")
            .send_activity(activity)
        {
            Ok(()) => PresenceState {
                connected: true,
                error: None,
            },
            Err(error) => {
                self.disconnect();
                PresenceState {
                    connected: false,
                    error: Some(format!("Could not update Discord: {error}")),
                }
            }
        }
    }

    fn clear_activity(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let _ = client.clear_activity();
        }
    }

    fn disconnect(&mut self) {
        self.clear_activity();
        if let Some(client) = self.client.as_mut() {
            let _ = client.close();
        }
        self.client = None;
        self.client_id.clear();
    }

    fn activity(
        &self,
        settings: &Settings,
        detection: &ClipStudioDetection,
        shared_screenshot_url: Option<&str>,
        procrastination_percent: Option<u8>,
    ) -> Value {
        let mut activity = Map::new();

        let rpc_name = rpc_name(settings, detection);
        activity.insert(
            "name".to_string(),
            Value::String(activity_text(&rpc_name, DEFAULT_RPC_NAME, 128)),
        );
        activity.insert(
            "type".to_string(),
            Value::Number(activity_type_value(&settings.activity_type).into()),
        );

        let mut details = if detection.focused {
            activity_text(&settings.presence_message, DEFAULT_PRESENCE_MESSAGE, 128)
        } else {
            activity_text(&settings.idle_message, DEFAULT_IDLE_MESSAGE, 128)
        };
        if !detection.focused && settings.show_procrastination_percent {
            details = with_procrastination_percent(details, procrastination_percent, 128);
        }
        activity.insert("details".to_string(), Value::String(details));

        if detection.focused {
            let state = if settings.show_document_name {
                detection
                    .document_title
                    .as_deref()
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| settings.state_text.trim())
            } else {
                settings.state_text.trim()
            };
            let mut state = activity_text(state, DEFAULT_STATE_TEXT, 128);
            if settings.show_procrastination_percent {
                state = with_procrastination_percent(state, procrastination_percent, 128);
            }
            activity.insert("state".to_string(), Value::String(state));
        }

        if let Some(timestamps) = self.timestamps(settings, detection.focused) {
            activity.insert("timestamps".to_string(), timestamps);
        }

        if let Some(assets) = assets(settings) {
            activity.insert("assets".to_string(), assets);
        }

        if detection.focused {
            if let Some(party) = party(settings) {
                activity.insert("party".to_string(), party);
            }
        }

        if let Some(buttons) = buttons(settings, shared_screenshot_url) {
            activity.insert("buttons".to_string(), buttons);
        }

        Value::Object(activity)
    }

    fn timestamps(&self, settings: &Settings, focused: bool) -> Option<Value> {
        if !settings.show_elapsed_time {
            return None;
        }

        let mut timestamps = Map::new();
        match settings.timestamp_mode.trim().to_ascii_lowercase().as_str() {
            "none" => None,
            "app" => {
                timestamps.insert("start".to_string(), json!(self.app_started_at));
                Some(Value::Object(timestamps))
            }
            "custom" => custom_timestamps(settings),
            _ if focused => {
                timestamps.insert("start".to_string(), json!(self.active_since?));
                Some(Value::Object(timestamps))
            }
            _ => None,
        }
    }
}

trait DiscordIpcActivityExt {
    fn send_activity(&mut self, activity_payload: Value) -> Result<(), Box<dyn std::error::Error>>;
}

impl DiscordIpcActivityExt for DiscordIpcClient {
    fn send_activity(&mut self, activity_payload: Value) -> Result<(), Box<dyn std::error::Error>> {
        let data = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity_payload
            },
            "nonce": format!("{}-{}", std::process::id(), now_unix())
        });
        self.send(data, 1)
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn activity_text(value: &str, fallback: &str, max_chars: usize) -> String {
    let text = value.trim();
    let text = if text.is_empty() { fallback } else { text };
    text.chars().take(max_chars).collect()
}

fn rpc_name(settings: &Settings, detection: &ClipStudioDetection) -> String {
    if settings.rpc_name_from_document {
        if let Some(document_title) = detection
            .document_title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
        {
            return document_title.to_string();
        }
    }

    settings.rpc_name.clone()
}

fn with_procrastination_percent(text: String, percent: Option<u8>, max_chars: usize) -> String {
    let Some(percent) = percent else {
        return text;
    };

    let suffix = format!(" ({percent}% procrastinated)");
    let mut combined = text;
    combined.push_str(&suffix);
    combined.chars().take(max_chars).collect()
}

fn optional_text(value: &str, max_chars: usize) -> Option<String> {
    let text = value.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.chars().take(max_chars).collect())
    }
}

fn activity_type_value(value: &str) -> u8 {
    match value.trim().to_ascii_lowercase().as_str() {
        "listening" => 2,
        "watching" => 3,
        "competing" => 5,
        DEFAULT_ACTIVITY_TYPE | _ => 0,
    }
}

fn valid_button_url(url: &str) -> Option<&str> {
    let url = url.trim();
    if url.len() <= 512 && (url.starts_with("https://") || url.starts_with("http://")) {
        Some(url)
    } else {
        None
    }
}

fn normalized_button_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let url = if url.contains("://") {
        url.to_string()
    } else {
        format!("https://{url}")
    };

    valid_button_url(&url).map(ToString::to_string)
}

fn push_custom_button(buttons: &mut Vec<Value>, label: &str, url: &str) {
    if buttons.len() >= 2 {
        return;
    }

    let Some(label) = optional_text(label, 32) else {
        return;
    };
    let Some(url) = normalized_button_url(url) else {
        return;
    };

    buttons.push(json!({
        "label": label,
        "url": url
    }));
}

fn configured_client_id(settings: &Settings) -> String {
    let client_id = settings.discord_client_id.trim();
    if client_id.is_empty() {
        DISCORD_CLIENT_ID.to_string()
    } else {
        client_id.to_string()
    }
}

fn assets(settings: &Settings) -> Option<Value> {
    let icon_key = activity_text(&settings.icon_key, DEFAULT_ICON_KEY, 128);
    let icon_text = activity_text(&settings.icon_text, DEFAULT_ICON_TEXT, 128);
    let small_icon_key = optional_text(&settings.small_icon_key, 128);
    let small_icon_text = optional_text(&settings.small_icon_text, 128);
    let mut assets = Map::new();

    if !icon_key.is_empty() {
        assets.insert("large_image".to_string(), Value::String(icon_key));
        if !icon_text.is_empty() {
            assets.insert("large_text".to_string(), Value::String(icon_text));
        }
    }

    if let Some(small_icon_key) = small_icon_key {
        assets.insert("small_image".to_string(), Value::String(small_icon_key));
        if let Some(small_icon_text) = small_icon_text {
            assets.insert("small_text".to_string(), Value::String(small_icon_text));
        }
    }

    if assets.is_empty() {
        None
    } else {
        Some(Value::Object(assets))
    }
}

fn buttons(settings: &Settings, shared_screenshot_url: Option<&str>) -> Option<Value> {
    let mut buttons = Vec::with_capacity(2);
    let button_label = activity_text(&settings.share_button_label, DEFAULT_SHARE_BUTTON_LABEL, 32);
    if let Some(url) = shared_screenshot_url.and_then(valid_button_url) {
        buttons.push(json!({
            "label": button_label,
            "url": url
        }));
    }

    push_custom_button(
        &mut buttons,
        &settings.button_1_label,
        &settings.button_1_url,
    );
    push_custom_button(
        &mut buttons,
        &settings.button_2_label,
        &settings.button_2_url,
    );

    if buttons.is_empty() {
        None
    } else {
        Some(Value::Array(buttons))
    }
}

fn party(settings: &Settings) -> Option<Value> {
    if settings.party_size == 0 || settings.party_max == 0 {
        return None;
    }

    let size = settings
        .party_size
        .min(settings.party_max)
        .min(i32::MAX as u32) as i32;
    let max = settings.party_max.min(i32::MAX as u32) as i32;
    Some(json!({
        "id": "clip-studio-presence",
        "size": [size, max]
    }))
}

fn custom_timestamps(settings: &Settings) -> Option<Value> {
    let start = settings.custom_timestamp_start;
    let end = settings.custom_timestamp_end;

    if start <= 0 && end <= 0 {
        return None;
    }

    let mut timestamps = Map::new();
    if start > 0 {
        timestamps.insert("start".to_string(), json!(start));
    }
    if end > 0 {
        timestamps.insert("end".to_string(), json!(end));
    }

    Some(Value::Object(timestamps))
}
