use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/Neroil/clip_studio_paint_discord_RPC/releases/latest";

#[derive(Clone, Debug, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub notes: Option<String>,
    pub message: String,
}

pub fn check_for_updates() -> Result<UpdateCheckResult, UpdateCheckError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = Client::builder()
        .user_agent("ClipStudioPresence/0.1")
        .http1_only()
        .build()?;

    let response = client
        .get(GITHUB_LATEST_RELEASE_URL)
        .header(USER_AGENT, "ClipStudioPresence/0.1")
        .header(ACCEPT, "application/vnd.github+json")
        .send()?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateCheckResult {
            current_version,
            latest_version: None,
            update_available: false,
            release_url: None,
            notes: None,
            message: "No GitHub releases exist yet.".to_string(),
        });
    }

    let status = response.status();
    if !status.is_success() {
        return Err(UpdateCheckError::GitHubStatus {
            status,
            body: response.text().unwrap_or_default(),
        });
    }

    let body = response.text()?;
    let release = serde_json::from_str::<GitHubRelease>(&body)?;
    let latest_version = clean_version(&release.tag_name);
    let update_available = version_is_newer(&latest_version, &current_version);
    let message = if update_available {
        format!("Version {latest_version} is available on GitHub.")
    } else {
        format!("You're up to date on version {current_version}.")
    };

    Ok(UpdateCheckResult {
        current_version,
        latest_version: Some(latest_version),
        update_available,
        release_url: Some(release.html_url),
        notes: release.body.filter(|body| !body.trim().is_empty()),
        message,
    })
}

fn clean_version(version: &str) -> String {
    version.trim().trim_start_matches(['v', 'V']).to_string()
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest = version_parts(latest);
    let current = version_parts(current);

    for index in 0..latest.len().max(current.len()) {
        let left = *latest.get(index).unwrap_or(&0);
        let right = *current.get(index).unwrap_or(&0);
        if left != right {
            return left > right;
        }
    }

    false
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split(|char: char| !char.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateCheckError {
    #[error("could not contact GitHub releases: {0}")]
    Request(#[from] reqwest::Error),
    #[error("GitHub release response was not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("GitHub returned HTTP {status} while checking releases: {body}")]
    GitHubStatus {
        status: reqwest::StatusCode,
        body: String,
    },
}
