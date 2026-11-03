# Omoi for Windows

Metadata/tag editing is in Details, exports are in Settings, and scheduling controls are in Sources. Optional playback setup is in **PLAYBACK.md**; launch **Run Playback.cmd** separately and install **omoi.user.js**. Capture does not start automatically. **ACCEPTANCE.md** records current checks and remaining acceptance.

1. Extract this ZIP to a folder.
2. Double-click **Run Omoi.cmd**. Keep the console open while using the app.
3. If the browser does not open, copy the URL printed in the console into Edge or another browser.
4. Choose **Sources**, check the browser profiles to import, select **Collect selected**, and confirm.
5. Use **History** to search, filter and open item details. **Settings** contains all five themes.

The default catalogue is `%LOCALAPPDATA%\omoi\history.sqlite`. Existing CLI data appears automatically. `OMOI_HOME` can select a different catalogue; it must be an absolute path. Starting the UI does not collect browser history or install a scheduled task.

The local server exits after ten idle minutes. Use the app's Refresh button to reload data. Reloading the entire browser tab clears its session; reopen the console's launch URL. The URL grants access to this local session, so keep it private.

If port 43822 is occupied, close the other Omoi console or run `omoi.exe ui --port 0 --open` in this folder. Appearance preferences are stored per browser origin, so an alternate port has separate preferences.

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

This package contains a locally verified development build, not a signed public release. `UI.md` describes behavior and request boundaries. `ACCEPTANCE.md` and `ui/acceptance/` record the current checks; `MEASUREMENTS.md` preserves earlier resource baselines and their limits. `BUILD.json` lists the source commit and file hashes. No personal browsing data is included.
