import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const fields = {
  presenceMessage: document.querySelector("#presence-message"),
  iconKey: document.querySelector("#icon-key"),
  showDocumentName: document.querySelector("#show-document-name"),
  showElapsedTime: document.querySelector("#show-elapsed-time"),
  onlyWhenFocused: document.querySelector("#only-when-focused"),
};

const statusNodes = {
  pill: document.querySelector("#connection-pill"),
  cspRunning: document.querySelector("#csp-running"),
  cspFocused: document.querySelector("#csp-focused"),
  documentTitle: document.querySelector("#document-title"),
  discordState: document.querySelector("#discord-state"),
  message: document.querySelector("#status-message"),
};

const form = document.querySelector("#settings-form");
const refreshButton = document.querySelector("#refresh-button");
let settingsHydrated = false;

function applySettings(settings) {
  fields.presenceMessage.value = settings.presence_message ?? "Drawing in Clip Studio Paint";
  fields.iconKey.value = settings.icon_key ?? "clip_studio_paint";
  fields.showDocumentName.checked = settings.show_document_name;
  fields.showElapsedTime.checked = settings.show_elapsed_time;
  fields.onlyWhenFocused.checked = settings.only_when_focused;
}

function readSettings() {
  return {
    presence_message: fields.presenceMessage.value.trim() || "Drawing in Clip Studio Paint",
    icon_key: fields.iconKey.value.trim() || "clip_studio_paint",
    show_document_name: fields.showDocumentName.checked,
    show_elapsed_time: fields.showElapsedTime.checked,
    only_when_focused: fields.onlyWhenFocused.checked,
  };
}

function boolText(value) {
  return value ? "Yes" : "No";
}

function setPill(status) {
  statusNodes.pill.className = "pill";

  if (status.discord_connected) {
    statusNodes.pill.textContent = "Presence active";
    statusNodes.pill.classList.add("good");
    return;
  }

  if (status.discord_error) {
    statusNodes.pill.textContent = "Needs attention";
    statusNodes.pill.classList.add("warn");
    return;
  }

  statusNodes.pill.textContent = "Idle";
  statusNodes.pill.classList.add("muted");
}

function renderStatus(status) {
  if (!settingsHydrated) {
    applySettings(status.settings);
    settingsHydrated = true;
  }

  statusNodes.cspRunning.textContent = boolText(status.clip_studio_running);
  statusNodes.cspFocused.textContent = boolText(status.clip_studio_focused);
  statusNodes.documentTitle.textContent = status.document_title || "Hidden or unavailable";
  statusNodes.discordState.textContent = status.discord_connected ? "Connected" : "Disconnected";
  statusNodes.message.textContent = status.discord_error || "";
  setPill(status);
}

async function refreshStatus() {
  try {
    const status = await invoke("get_status");
    renderStatus(status);
  } catch (error) {
    statusNodes.message.textContent = String(error);
    statusNodes.pill.textContent = "Error";
    statusNodes.pill.className = "pill warn";
  }
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  statusNodes.message.textContent = "Saving...";
  try {
    const status = await invoke("save_settings", { settings: readSettings() });
    applySettings(status.settings);
    renderStatus(status);
  } catch (error) {
    statusNodes.message.textContent = String(error);
  }
});

refreshButton.addEventListener("click", refreshStatus);

refreshStatus();
setInterval(refreshStatus, 3000);
