# Clip Studio Presence

A small Tauri + Rust desktop app that publishes Discord Rich Presence while Clip Studio Paint is running.

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
assets in the Discord Developer Portal for the project application, then use their asset keys in the
app's icon setting.
