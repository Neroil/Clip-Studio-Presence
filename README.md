# Clip Studio Presence

A small Tauri + Rust desktop app that publishes Discord Rich Presence while Clip Studio Paint is running.

## Discord setup

The Discord application is created once for the project. Users of the app do not need to make their
own Discord Developer Application.

1. Open the [Discord Developer Portal](https://discord.com/developers/applications).
2. Create or open the project application.
3. In **General Information**, copy the **Application ID**.
4. If the ID changes, update `DISCORD_CLIENT_ID` in `src-tauri/src/app_config.rs`.
5. In the Rich Presence art/assets section, upload the icons used by the app.
6. Name the uploaded asset keys:

```text
icon_1
icon_2
icon_3
```

Discord may take a few minutes to make new art assets available in Rich Presence.

The **Capture & Share** button does not need Discord portal setup. It screenshots the Clip Studio
Paint window, uploads the image, and updates the Rich Presence button URL at runtime.

## Development

Install dependencies:

```powershell
npm install
```

Run the app:

```powershell
npm run tauri dev
```

The app uses the bundled Discord application ID for Rich Presence. Upload Rich Presence image
assets named `icon_1`, `icon_2`, and `icon_3` in the Discord Developer Portal for the project
application.

Use **Capture & Share** to screenshot the current Clip Studio Paint window, upload it, and attach a
Discord activity button labeled **See what I'm working on**.
