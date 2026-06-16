import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const fields = {
  discordClientId: document.querySelector("#discord-client-id"),
  activityType: document.querySelector("#activity-type"),
  rpcName: document.querySelector("#rpc-name"),
  rpcNameFromDocument: document.querySelector("#rpc-name-from-document"),
  presenceMessage: document.querySelector("#presence-message"),
  idleMessage: document.querySelector("#idle-message"),
  stateText: document.querySelector("#state-text"),
  iconKey: document.querySelector("#icon-key"),
  iconText: document.querySelector("#icon-text"),
  smallIconKey: document.querySelector("#small-icon-key"),
  smallIconText: document.querySelector("#small-icon-text"),
  button1Label: document.querySelector("#button-1-label"),
  button1Url: document.querySelector("#button-1-url"),
  button2Label: document.querySelector("#button-2-label"),
  button2Url: document.querySelector("#button-2-url"),
  shareButtonLabel: document.querySelector("#share-button-label"),
  timestampMode: document.querySelector("#timestamp-mode"),
  customTimestampStart: document.querySelector("#custom-timestamp-start"),
  customTimestampEnd: document.querySelector("#custom-timestamp-end"),
  partySize: document.querySelector("#party-size"),
  partyMax: document.querySelector("#party-max"),
  showDocumentName: document.querySelector("#show-document-name"),
  showElapsedTime: document.querySelector("#show-elapsed-time"),
  showProcrastinationPercent: document.querySelector("#show-procrastination-percent"),
};

const statusNodes = {
  pill: document.querySelector("#connection-pill"),
  cspRunning: document.querySelector("#csp-running"),
  cspFocused: document.querySelector("#csp-focused"),
  documentTitle: document.querySelector("#document-title"),
  discordState: document.querySelector("#discord-state"),
  procrastinationPercent: document.querySelector("#procrastination-percent"),
  sharedScreenshot: document.querySelector("#shared-screenshot"),
  message: document.querySelector("#status-message"),
};

const form = document.querySelector("#settings-form");
const refreshButton = document.querySelector("#refresh-button");
const captureButton = document.querySelector("#capture-button");
const useCurrentFileButton = document.querySelector("#use-current-file-button");
let settingsHydrated = false;
let currentStatus = null;

function applySettings(settings) {
  fields.discordClientId.value = settings.discord_client_id ?? "1516410830063796294";
  fields.activityType.value = settings.activity_type ?? "playing";
  fields.rpcName.value = settings.rpc_name ?? "Clip Studio Paint";
  fields.rpcNameFromDocument.checked = settings.rpc_name_from_document ?? false;
  fields.presenceMessage.value = settings.presence_message ?? "Drawing in Clip Studio Paint";
  fields.idleMessage.value = settings.idle_message ?? "Procrastinating teehee";
  fields.stateText.value = settings.state_text ?? "Working on an illustration";
  fields.iconKey.value = settings.icon_key ?? "icon_1";
  fields.iconText.value = settings.icon_text ?? "Clip Studio Paint";
  fields.smallIconKey.value = settings.small_icon_key ?? "";
  fields.smallIconText.value = settings.small_icon_text ?? "";
  fields.button1Label.value = settings.button_1_label ?? "";
  fields.button1Url.value = settings.button_1_url ?? "";
  fields.button2Label.value = settings.button_2_label ?? "";
  fields.button2Url.value = settings.button_2_url ?? "";
  fields.shareButtonLabel.value = settings.share_button_label ?? "See what I'm working on";
  fields.timestampMode.value = settings.timestamp_mode ?? "activity";
  fields.customTimestampStart.value = unixToDateTimeLocal(settings.custom_timestamp_start);
  fields.customTimestampEnd.value = unixToDateTimeLocal(settings.custom_timestamp_end);
  fields.partySize.value = settings.party_size ?? 0;
  fields.partyMax.value = settings.party_max ?? 0;
  fields.showDocumentName.checked = settings.show_document_name;
  fields.showElapsedTime.checked = settings.show_elapsed_time;
  fields.showProcrastinationPercent.checked = settings.show_procrastination_percent ?? true;
  updateCustomTimestampVisibility();
}

function readSettings() {
  return {
    discord_client_id: fields.discordClientId.value.trim() || "1516410830063796294",
    activity_type: fields.activityType.value,
    rpc_name: fields.rpcName.value.trim() || "Clip Studio Paint",
    rpc_name_from_document: fields.rpcNameFromDocument.checked,
    presence_message: fields.presenceMessage.value.trim() || "Drawing in Clip Studio Paint",
    idle_message: fields.idleMessage.value.trim() || "Procrastinating teehee",
    state_text: fields.stateText.value.trim() || "Working on an illustration",
    icon_key: fields.iconKey.value.trim() || "icon_1",
    icon_text: fields.iconText.value.trim() || "Clip Studio Paint",
    small_icon_key: fields.smallIconKey.value.trim(),
    small_icon_text: fields.smallIconText.value.trim(),
    button_1_label: fields.button1Label.value.trim(),
    button_1_url: fields.button1Url.value.trim(),
    button_2_label: fields.button2Label.value.trim(),
    button_2_url: fields.button2Url.value.trim(),
    share_button_label: fields.shareButtonLabel.value.trim() || "See what I'm working on",
    timestamp_mode: fields.timestampMode.value,
    custom_timestamp_start: dateTimeLocalToUnix(fields.customTimestampStart.value),
    custom_timestamp_end: dateTimeLocalToUnix(fields.customTimestampEnd.value),
    party_size: clampNumber(fields.partySize.value, 0, 2147483647),
    party_max: clampNumber(fields.partyMax.value, 0, 2147483647),
    show_document_name: fields.showDocumentName.checked,
    show_elapsed_time: fields.showElapsedTime.checked,
    show_procrastination_percent: fields.showProcrastinationPercent.checked,
    only_when_focused: true,
  };
}

function clampNumber(value, min, max) {
  const number = Number.parseInt(value, 10);
  if (!Number.isFinite(number)) {
    return min;
  }
  return Math.min(Math.max(number, min), max);
}

function unixToDateTimeLocal(value) {
  if (!value || value <= 0) {
    return "";
  }

  const date = new Date(value * 1000);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
  return local.toISOString().slice(0, 16);
}

function dateTimeLocalToUnix(value) {
  if (!value) {
    return 0;
  }

  return Math.floor(new Date(value).getTime() / 1000);
}

function updateCustomTimestampVisibility() {
  const visible = fields.timestampMode.value === "custom";
  document.querySelector("#custom-timestamp-fields").hidden = !visible;
}

function setPill(status) {
  statusNodes.pill.className = "pill";

  if (status.discord_connected) {
    statusNodes.pill.textContent = status.clip_studio_focused ? "Live" : "Away";
    statusNodes.pill.classList.add("good");
    return;
  }

  if (status.discord_error) {
    statusNodes.pill.textContent = "Check Discord";
    statusNodes.pill.classList.add("warn");
    return;
  }

  statusNodes.pill.textContent = "Off";
  statusNodes.pill.classList.add("muted");
}

function renderStatus(status) {
  currentStatus = status;

  if (!settingsHydrated) {
    applySettings(status.settings);
    settingsHydrated = true;
  }

  statusNodes.cspRunning.textContent = status.clip_studio_running ? "Open" : "Closed";
  statusNodes.cspFocused.textContent = status.clip_studio_focused ? "In Paint" : "Away";
  statusNodes.documentTitle.textContent = status.document_title || "Hidden or unavailable";
  statusNodes.discordState.textContent = status.discord_connected ? "Connected" : "Disconnected";
  statusNodes.procrastinationPercent.textContent =
    status.procrastination_percent == null ? "0%" : `${status.procrastination_percent}%`;
  renderSharedScreenshot(status.shared_screenshot_url);
  statusNodes.message.textContent = status.discord_error || "";
  setPill(status);
}

function useCurrentFileName() {
  const documentTitle = currentStatus?.document_title?.trim();
  if (!documentTitle) {
    statusNodes.message.textContent =
      "Focus Clip Studio Paint first so I can read the current file name.";
    return;
  }

  fields.rpcName.value = documentTitle;
  statusNodes.message.textContent = "RPC name copied from the current file.";
}

function renderSharedScreenshot(url) {
  statusNodes.sharedScreenshot.textContent = "";

  if (!url) {
    statusNodes.sharedScreenshot.textContent = "Not captured yet";
    return;
  }

  const link = document.createElement("a");
  link.href = url;
  link.target = "_blank";
  link.rel = "noreferrer";
  link.textContent = url;
  statusNodes.sharedScreenshot.append(link);
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

async function captureAndShare() {
  captureButton.disabled = true;
  captureButton.textContent = "Capturing...";
  statusNodes.message.textContent = "Capturing Clip Studio Paint and uploading...";

  try {
    const status = await invoke("capture_and_share");
    renderStatus(status);
    statusNodes.message.textContent =
      "Shared screenshot updated. Discord will refresh the button shortly.";
  } catch (error) {
    statusNodes.message.textContent = String(error);
  } finally {
    captureButton.disabled = false;
    captureButton.textContent = "Capture & Share";
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
captureButton.addEventListener("click", captureAndShare);
useCurrentFileButton.addEventListener("click", useCurrentFileName);
fields.timestampMode.addEventListener("change", updateCustomTimestampVisibility);

refreshStatus();
setInterval(refreshStatus, 3000);
