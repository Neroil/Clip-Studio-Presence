import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const fields = {
  discordClientId: document.querySelector("#discord-client-id"),
  activityType: document.querySelector("#activity-type"),
  statusDisplayType: document.querySelector("#status-display-type"),
  rpcName: document.querySelector("#rpc-name"),
  rpcNameFromDocument: document.querySelector("#rpc-name-from-document"),
  presenceMessage: document.querySelector("#presence-message"),
  presenceUrl: document.querySelector("#presence-url"),
  idleMessage: document.querySelector("#idle-message"),
  stateText: document.querySelector("#state-text"),
  stateUrl: document.querySelector("#state-url"),
  iconKey: document.querySelector("#icon-key"),
  iconText: document.querySelector("#icon-text"),
  iconUrl: document.querySelector("#icon-url"),
  smallIconKey: document.querySelector("#small-icon-key"),
  smallIconText: document.querySelector("#small-icon-text"),
  smallIconUrl: document.querySelector("#small-icon-url"),
  button1Label: document.querySelector("#button-1-label"),
  button1Url: document.querySelector("#button-1-url"),
  button2Label: document.querySelector("#button-2-label"),
  button2Url: document.querySelector("#button-2-url"),
  applyScreenshotLut: document.querySelector("#apply-screenshot-lut"),
  screenshotLutPath: document.querySelector("#screenshot-lut-path"),
  autoCaptureScreenshot: document.querySelector("#auto-capture-screenshot"),
  autoCaptureInitialDelaySeconds: document.querySelector("#auto-capture-initial-delay-seconds"),
  autoCaptureIntervalSeconds: document.querySelector("#auto-capture-interval-seconds"),
  timestampMode: document.querySelector("#timestamp-mode"),
  customTimestampStart: document.querySelector("#custom-timestamp-start"),
  customTimestampEnd: document.querySelector("#custom-timestamp-end"),
  partySize: document.querySelector("#party-size"),
  partyMax: document.querySelector("#party-max"),
  showDocumentName: document.querySelector("#show-document-name"),
  showElapsedTime: document.querySelector("#show-elapsed-time"),
  showProcrastinationPercent: document.querySelector("#show-procrastination-percent"),
  startOnBoot: document.querySelector("#start-on-boot"),
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
  update: document.querySelector("#update-status"),
  previewStatus: document.querySelector("#preview-status"),
  previewLargeImage: document.querySelector("#preview-large-image"),
  previewSmallImage: document.querySelector("#preview-small-image"),
  previewName: document.querySelector("#preview-name"),
  previewDetails: document.querySelector("#preview-details"),
  previewState: document.querySelector("#preview-state"),
  previewTime: document.querySelector("#preview-time"),
  previewButtons: document.querySelector("#preview-buttons"),
};

const form = document.querySelector("#settings-form");
const refreshButton = document.querySelector("#refresh-button");
const captureButton = document.querySelector("#capture-button");
const checkUpdatesButton = document.querySelector("#check-updates-button");
const useCurrentFileButton = document.querySelector("#use-current-file-button");
let settingsHydrated = false;
let currentStatus = null;

function applySettings(settings) {
  fields.discordClientId.value = settings.discord_client_id ?? "1516410830063796294";
  fields.activityType.value = settings.activity_type ?? "playing";
  fields.statusDisplayType.value = settings.status_display_type ?? "name";
  fields.rpcName.value = settings.rpc_name ?? "Clip Studio Paint";
  fields.rpcNameFromDocument.checked = settings.rpc_name_from_document ?? false;
  fields.presenceMessage.value = settings.presence_message ?? "Drawing in Clip Studio Paint";
  fields.presenceUrl.value = settings.presence_url ?? "";
  fields.idleMessage.value = settings.idle_message ?? "Procrastinating teehee";
  fields.stateText.value = settings.state_text ?? "Working on an illustration";
  fields.stateUrl.value = settings.state_url ?? "";
  fields.iconKey.value = settings.icon_key ?? "icon_1";
  fields.iconText.value = settings.icon_text ?? "Clip Studio Paint";
  fields.iconUrl.value = settings.icon_url ?? "";
  fields.smallIconKey.value = settings.small_icon_key ?? "";
  fields.smallIconText.value = settings.small_icon_text ?? "";
  fields.smallIconUrl.value = settings.small_icon_url ?? "";
  fields.button1Label.value = settings.button_1_label ?? "";
  fields.button1Url.value = settings.button_1_url ?? "";
  fields.button2Label.value = settings.button_2_label ?? "";
  fields.button2Url.value = settings.button_2_url ?? "";
  fields.applyScreenshotLut.checked = settings.apply_screenshot_lut ?? false;
  fields.screenshotLutPath.value = settings.screenshot_lut_path ?? "";
  fields.autoCaptureScreenshot.checked = settings.auto_capture_screenshot ?? false;
  fields.autoCaptureInitialDelaySeconds.value =
    settings.auto_capture_initial_delay_seconds ?? 30;
  fields.autoCaptureIntervalSeconds.value = settings.auto_capture_interval_seconds ?? 300;
  fields.timestampMode.value = settings.timestamp_mode ?? "activity";
  fields.customTimestampStart.value = unixToDateTimeLocal(settings.custom_timestamp_start);
  fields.customTimestampEnd.value = unixToDateTimeLocal(settings.custom_timestamp_end);
  fields.partySize.value = settings.party_size ?? 0;
  fields.partyMax.value = settings.party_max ?? 0;
  fields.showDocumentName.checked = settings.show_document_name;
  fields.showElapsedTime.checked = settings.show_elapsed_time;
  fields.showProcrastinationPercent.checked = settings.show_procrastination_percent ?? true;
  fields.startOnBoot.checked = settings.start_on_boot ?? false;
  updateCustomTimestampVisibility();
  updateScreenshotLutVisibility();
  updateAutoCaptureVisibility();
  updatePreview();
}

function readSettings() {
  return {
    discord_client_id: fields.discordClientId.value.trim() || "1516410830063796294",
    activity_type: fields.activityType.value,
    status_display_type: fields.statusDisplayType.value,
    rpc_name: fields.rpcName.value.trim() || "Clip Studio Paint",
    rpc_name_from_document: fields.rpcNameFromDocument.checked,
    presence_message: fields.presenceMessage.value.trim() || "Drawing in Clip Studio Paint",
    presence_url: fields.presenceUrl.value.trim(),
    idle_message: fields.idleMessage.value.trim() || "Procrastinating teehee",
    state_text: fields.stateText.value.trim() || "Working on an illustration",
    state_url: fields.stateUrl.value.trim(),
    icon_key: fields.iconKey.value.trim() || "icon_1",
    icon_text: fields.iconText.value.trim() || "Clip Studio Paint",
    icon_url: fields.iconUrl.value.trim(),
    small_icon_key: fields.smallIconKey.value.trim(),
    small_icon_text: fields.smallIconText.value.trim(),
    small_icon_url: fields.smallIconUrl.value.trim(),
    button_1_label: fields.button1Label.value.trim(),
    button_1_url: fields.button1Url.value.trim(),
    button_2_label: fields.button2Label.value.trim(),
    button_2_url: fields.button2Url.value.trim(),
    apply_screenshot_lut: fields.applyScreenshotLut.checked,
    screenshot_lut_path: fields.screenshotLutPath.value.trim(),
    auto_capture_screenshot: fields.autoCaptureScreenshot.checked,
    auto_capture_initial_delay_seconds: clampNumber(
      fields.autoCaptureInitialDelaySeconds.value,
      1,
      86400,
    ),
    auto_capture_interval_seconds: clampNumber(fields.autoCaptureIntervalSeconds.value, 1, 86400),
    timestamp_mode: fields.timestampMode.value,
    custom_timestamp_start: dateTimeLocalToUnix(fields.customTimestampStart.value),
    custom_timestamp_end: dateTimeLocalToUnix(fields.customTimestampEnd.value),
    party_size: clampNumber(fields.partySize.value, 0, 2147483647),
    party_max: clampNumber(fields.partyMax.value, 0, 2147483647),
    show_document_name: fields.showDocumentName.checked,
    show_elapsed_time: fields.showElapsedTime.checked,
    show_procrastination_percent: fields.showProcrastinationPercent.checked,
    start_on_boot: fields.startOnBoot.checked,
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

function updateScreenshotLutVisibility() {
  fields.screenshotLutPath.disabled = !fields.applyScreenshotLut.checked;
}

function updateAutoCaptureVisibility() {
  const disabled = !fields.autoCaptureScreenshot.checked;
  fields.autoCaptureInitialDelaySeconds.disabled = disabled;
  fields.autoCaptureIntervalSeconds.disabled = disabled;
}

function activityText(value, fallback, maxLength) {
  const text = value.trim() || fallback;
  return text.slice(0, maxLength);
}

function optionalText(value, maxLength) {
  const text = value.trim();
  return text ? text.slice(0, maxLength) : "";
}

function withProcrastinationPercent(text) {
  if (!fields.showProcrastinationPercent.checked) {
    return text;
  }

  const percent = currentStatus?.procrastination_percent ?? 0;
  return `${text} (${percent}% procrastinated)`.slice(0, 128);
}

function previewDocumentTitle() {
  return currentStatus?.document_title?.trim() || "";
}

function previewRpcName() {
  if (fields.rpcNameFromDocument.checked && previewDocumentTitle()) {
    return previewDocumentTitle();
  }

  return activityText(fields.rpcName.value, "Clip Studio Paint", 128);
}

function previewStateText(focused) {
  if (!focused) {
    return "";
  }

  const state = fields.showDocumentName.checked && previewDocumentTitle()
    ? previewDocumentTitle()
    : activityText(fields.stateText.value, "Working on an illustration", 128);
  return withProcrastinationPercent(state);
}

function previewDetailsText(focused) {
  const text = focused
    ? activityText(fields.presenceMessage.value, "Drawing in Clip Studio Paint", 128)
    : activityText(fields.idleMessage.value, "Procrastinating teehee", 128);

  return focused ? text : withProcrastinationPercent(text);
}

function previewImageText(value, fallback) {
  const text = activityText(value, fallback, 128);
  return text
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toUpperCase()
    .slice(0, 2) || "CS";
}

function previewButtonLabels() {
  const labels = [];
  if (currentStatus?.shared_screenshot_url) {
    labels.push("See what I'm working on");
    const button2 = previewButtonLabel(fields.button2Label.value, fields.button2Url.value);
    if (button2) {
      labels.push(button2);
    }
  } else {
    const button1 = previewButtonLabel(fields.button1Label.value, fields.button1Url.value);
    const button2 = previewButtonLabel(fields.button2Label.value, fields.button2Url.value);
    if (button1) {
      labels.push(button1);
    }
    if (button2) {
      labels.push(button2);
    }
  }

  return labels.slice(0, 2);
}

function previewButtonLabel(label, url) {
  if (!processedUrl(url)) {
    return "";
  }

  return optionalText(label, 32);
}

function processedUrl(url) {
  const text = url.trim();
  if (!text) {
    return "";
  }

  const normalized = text.includes("://") ? text : `https://${text}`;
  if (normalized.length > 512) {
    return "";
  }

  return normalized.startsWith("https://") || normalized.startsWith("http://") ? normalized : "";
}

function updatePreview() {
  const focused = currentStatus?.clip_studio_focused ?? true;
  const running = currentStatus?.clip_studio_running ?? true;
  statusNodes.previewStatus.textContent = running ? (focused ? "Focused" : "Away") : "Closed";
  statusNodes.previewName.textContent = previewRpcName();
  statusNodes.previewDetails.textContent = previewDetailsText(focused);

  const state = previewStateText(focused);
  statusNodes.previewState.textContent = state || "No state while away";
  statusNodes.previewState.hidden = !state;
  statusNodes.previewTime.textContent = fields.showElapsedTime.checked
    ? timestampPreviewText(focused)
    : "Timestamp hidden";

  statusNodes.previewLargeImage.textContent = previewImageText(fields.iconText.value, "Clip Studio Paint");
  const smallIcon = optionalText(fields.smallIconKey.value, 128);
  statusNodes.previewSmallImage.hidden = !smallIcon;
  statusNodes.previewSmallImage.textContent = previewImageText(fields.smallIconText.value, smallIcon);

  const buttons = previewButtonLabels();
  statusNodes.previewButtons.textContent = "";
  statusNodes.previewButtons.hidden = buttons.length === 0;
  for (const label of buttons) {
    const node = document.createElement("span");
    node.className = "preview-button";
    node.textContent = label;
    statusNodes.previewButtons.append(node);
  }
}

function timestampPreviewText(focused) {
  switch (fields.timestampMode.value) {
    case "none":
      return "Timestamp hidden";
    case "app":
      return "Since app start";
    case "custom":
      if (fields.customTimestampStart.value && fields.customTimestampEnd.value) {
        return "Custom time range";
      }
      if (fields.customTimestampStart.value) {
        return "Since custom start";
      }
      if (fields.customTimestampEnd.value) {
        return "Until custom end";
      }
      return "Custom timestamp";
    default:
      return focused ? "Actual drawing time" : "Procrastinating since away";
  }
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
  if (status.discord_error) {
    statusNodes.message.textContent = status.discord_error;
  } else if (status.auto_capture_uploading) {
    statusNodes.message.textContent = "Auto capture is uploading a screenshot...";
  } else if (status.auto_capture_error) {
    statusNodes.message.textContent = `Auto capture failed: ${status.auto_capture_error}`;
  } else {
    statusNodes.message.textContent = "";
  }
  setPill(status);
  updatePreview();
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
  updatePreview();
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

function renderUpdateResult(result) {
  statusNodes.update.textContent = "";

  if (!result.update_available || !result.release_url) {
    statusNodes.update.textContent = result.message;
    return;
  }

  const link = document.createElement("a");
  link.href = result.release_url;
  link.target = "_blank";
  link.rel = "noreferrer";
  link.textContent = "Open release";

  statusNodes.update.append(`${result.message} `, link);
}

function errorMessage(error) {
  if (error instanceof Error) {
    return error.message || String(error);
  }

  if (typeof error === "string") {
    return error;
  }

  return String(error);
}

function showError(error) {
  const message = `Capture & Share failed: ${errorMessage(error)}`;
  statusNodes.message.textContent = message;
  window.alert(message);
}

async function refreshStatus() {
  try {
    const status = await invoke("get_status");
    renderStatus(status);
  } catch (error) {
    showError(error);
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
    showError(error);
  } finally {
    captureButton.disabled = false;
    captureButton.textContent = "Capture & Share";
  }
}

async function checkForUpdates() {
  checkUpdatesButton.disabled = true;
  checkUpdatesButton.textContent = "Checking...";
  statusNodes.update.textContent = "Checking GitHub releases...";

  try {
    const result = await invoke("check_for_updates");
    renderUpdateResult(result);
  } catch (error) {
    statusNodes.update.textContent = `Update check failed: ${errorMessage(error)}`;
  } finally {
    checkUpdatesButton.disabled = false;
    checkUpdatesButton.textContent = "Check for Updates";
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
checkUpdatesButton.addEventListener("click", checkForUpdates);
useCurrentFileButton.addEventListener("click", useCurrentFileName);
fields.timestampMode.addEventListener("change", updateCustomTimestampVisibility);
fields.applyScreenshotLut.addEventListener("change", updateScreenshotLutVisibility);
fields.autoCaptureScreenshot.addEventListener("change", updateAutoCaptureVisibility);
form.addEventListener("input", updatePreview);
form.addEventListener("change", updatePreview);

refreshStatus();
updateScreenshotLutVisibility();
updateAutoCaptureVisibility();
updatePreview();
setInterval(refreshStatus, 3000);
