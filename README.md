# Clip Studio Presence

A small Tauri + Rust desktop app that publishes Discord Rich Presence while Clip Studio Paint is running.

## Discord setup

The app ships with a bundled Discord application ID, so users do not need to make their own Discord
Developer Application. For full CustomRP-style control over the bold app name and asset library,
create your own application and paste its ID into the app settings.

1. Open the [Discord Developer Portal](https://discord.com/developers/applications).
2. Create or open the project application.
3. In **General Information**, copy the **Application ID**.
4. Set the application **Name** to the title you want Discord to show in bold.
5. Paste the ID into **Discord application ID** in the app settings, or update
   `DISCORD_CLIENT_ID` in `src-tauri/src/app_config.rs` to change the bundled fallback.
6. In the Rich Presence art/assets section, upload the icons used by the app.
7. Name the uploaded asset keys:

```text
icon_1
icon_2
icon_3
```

Discord may take a few minutes to make new art assets available in Rich Presence.

The **Capture & Share** button does not need Discord portal setup. It screenshots the Clip Studio
Paint window, can optionally apply an OBS-style PNG LUT before upload, uploads the image to Uguu, and
updates the Rich Presence button URL at runtime.
By default, the app also auto-captures after Clip Studio Paint has been focused for 30 seconds and
then every 5 minutes while focus stays in Clip Studio Paint. Leaving Clip Studio Paint resets the
auto-capture timers.

The app settings can customize the Rich Presence activity label, details text, state text, large and
small image keys, image hover text, timestamp mode, party size, and up to two buttons. If **Show
document name** is enabled, the state line uses the active Clip Studio Paint window title when
available and falls back to the configured state text.

Discord allows a maximum of two Rich Presence buttons. When **Capture & Share** has a screenshot URL,
that share button uses the first slot and the app fills any remaining slot with custom buttons.

## Development

Install dependencies:

```powershell
npm install
```

Run the app:

```powershell
npm run tauri dev
```

The app uses the bundled Discord application ID unless the settings specify another one. Upload Rich
Presence image assets named `icon_1`, `icon_2`, and `icon_3` in the Discord Developer Portal for the
project application, or enter any custom asset keys you upload.

Use **Capture & Share** to screenshot the current Clip Studio Paint window, upload it, and attach a
Discord activity button labeled **See what I'm working on**.
