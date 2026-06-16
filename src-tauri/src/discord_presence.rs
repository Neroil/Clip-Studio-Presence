use crate::{
    app_config::{
        DEFAULT_ICON_KEY, DEFAULT_PRESENCE_MESSAGE, DISCORD_CLIENT_ID, SHARE_BUTTON_LABEL,
    },
    app_state::Settings,
    clip_studio::ClipStudioDetection,
};
use discord_rich_presence::{
    activity::{Activity, Assets, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct PresenceState {
    pub connected: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct PresenceClient {
    client: Option<DiscordIpcClient>,
    client_id: String,
    active_since: Option<i64>,
}

impl PresenceClient {
    pub fn sync(
        &mut self,
        settings: &Settings,
        detection: &ClipStudioDetection,
        shared_screenshot_url: Option<&str>,
    ) -> PresenceState {
        let should_show = detection.running && (!settings.only_when_focused || detection.focused);
        if !should_show {
            self.active_since = None;
            self.clear_activity();
            return PresenceState::default();
        }

        if self.client_id != DISCORD_CLIENT_ID || self.client.is_none() {
            self.disconnect();
            self.client_id = DISCORD_CLIENT_ID.to_string();

            match DiscordIpcClient::new(DISCORD_CLIENT_ID).and_then(|mut client| {
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

        let details = activity_text(&settings.presence_message, DEFAULT_PRESENCE_MESSAGE);
        let mut activity = Activity::new().details(details.as_str());

        let state = if settings.show_document_name {
            detection
                .document_title
                .as_deref()
                .filter(|title| !title.is_empty())
                .unwrap_or("Working on an illustration")
        } else {
            "Making art"
        };
        activity = activity.state(state);

        if settings.show_elapsed_time {
            if let Some(started_at) = self.active_since {
                activity = activity.timestamps(Timestamps::new().start(started_at));
            }
        }

        let icon_key = activity_text(&settings.icon_key, DEFAULT_ICON_KEY);
        if !icon_key.is_empty() {
            activity = activity.assets(
                Assets::new()
                    .large_image(icon_key.as_str())
                    .large_text("Clip Studio Paint"),
            );
        }

        if let Some(url) = shared_screenshot_url.and_then(valid_button_url) {
            activity = activity.buttons(vec![Button::new(SHARE_BUTTON_LABEL, url)]);
        }

        match self
            .client
            .as_mut()
            .expect("presence client should be connected")
            .set_activity(activity)
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
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn activity_text(value: &str, fallback: &str) -> String {
    let text = value.trim();
    let text = if text.is_empty() { fallback } else { text };
    text.chars().take(128).collect()
}

fn valid_button_url(url: &str) -> Option<&str> {
    let url = url.trim();
    if url.len() <= 512 && (url.starts_with("https://") || url.starts_with("http://")) {
        Some(url)
    } else {
        None
    }
}
