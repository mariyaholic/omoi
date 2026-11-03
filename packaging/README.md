# Omoi for Windows

Metadata/tag editing is in Details, exports are in Settings, and scheduling controls are in Sources. Optional playback setup is in **PLAYBACK.md**; launch **Run Playback.cmd** separately and install **omoi.user.js**. Capture does not start automatically. **ACCEPTANCE.md** records the scoped verification summary.

1. Extract this ZIP to a folder.
2. Double-click **omoi-desktop.exe** or **Run Omoi.cmd**.
3. Omoi opens in its own window. Closing it shuts down its local UI server.
4. Choose **Sources**, check the browser profiles to import, select **Collect selected**, and confirm.
5. Use **History** to search, filter and open item details. **Settings** contains all five themes.

The default catalogue is `%LOCALAPPDATA%\omoi\history.sqlite`. Existing CLI data appears automatically. `OMOI_HOME` can select a different catalogue; it must be an absolute path. Starting the UI does not collect browser history or install a scheduled task.

Use the app's Refresh button to reload data. The desktop window restores its session when reloaded and keeps appearance settings in its own WebView2 profile. All five themes remain available. Original video links open in your default browser.

The desktop uses loopback port 43823. If Omoi is already open, use its existing window. The browser interface remains available through `omoi.exe ui --port 0 --open`; its session and appearance settings are separate.

Keep `WebView2Loader.dll` beside `omoi-desktop.exe`. The desktop uses the Microsoft Edge WebView2 Runtime supplied with Windows 11. Browser data is stored under the catalogue's `webview` folder, separate from your normal browser profiles.

The same exports and scheduling controls are available through terminal commands:

```text
omoi.exe export --json --out history.json
omoi.exe export --csv --out history.csv
omoi.exe schedule status
omoi.exe schedule install
omoi.exe schedule remove
```

Keep both executables in a stable folder before installing a task. The task uses `config.toml` settings, not the UI's temporary profile selection.

Deletion permanently removes an item and its catalogue records. It does not delete browser history or video files. Browser openings and approximate tab time are not proof of playback. Suggestions use edited or captured tags/durations/sessions; shelves may be empty when required data is absent. Remote metadata APIs are not connected.

This package contains a locally verified development build, not a signed public release. `UI.md` describes behavior and request boundaries. `ACCEPTANCE.md` and `MEASUREMENTS.md` summarize completed verification and resources. Detailed acceptance logs and follow-up are retained locally. `BUILD.json` lists the source commit and file hashes. No personal browsing data is included.
