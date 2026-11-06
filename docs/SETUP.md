# Install and set up Omoi

These instructions describe the unsigned development build on `feat/guided-setup`.
History works immediately; playback is optional. Nothing is sent to a hosted service.
The companion is not published in the Edge store yet.

## History only

Run the per-user installer, or extract the portable ZIP and open `omoi-desktop.exe`.
Keep `WebView2Loader.dll` beside it. If the app reports a missing WebView2 runtime,
install Microsoft's [Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
and reopen Omoi. Omoi does not download or install this dependency silently.

Choose Sources, select profiles, then Collect selected. Automatic collection is a
separate confirmation: enabling it saves exactly the selected profile IDs. The task
runs at logon and hourly under your user account, subject to its existing battery
and overlap policy. Select a new scope and enable again to change it. Disable removes
the task without deleting history. CLI `collect` retains its configured scope;
`collect-scheduled` and the Windows helper require the saved selection and fail
closed when none exists. An old task must be enabled again in Sources to save scope.

All five themes remain in Settings. Appearance stays in the app's WebView2 profile.
Default data: `%LOCALAPPDATA%\omoi`. A custom `OMOI_HOME` must be absolute. Starting
Omoi never collects browser history or installs scheduled tasks by itself.

## Guided playback

In Settings, select Enable capture. Omoi starts and owns the loopback listener,
restores that choice next launch, drains durable pending events, and refreshes
History/Suggestions when materialized playback changes. Keep the app open by default.
No separate Run Playback launcher or Process playback click is required.

The UI distinguishes client contact, event receipt, and recorded sessions. Opening a
store page, exporting a file or completing a dialog does not establish readiness.
Client contact expires after 45 seconds. Old userscripts can send events but have an
unknown client version. Retry retains setup progress. A port collision asks you to
close/repair the other listener yourself; Omoi never kills it or rotates its token.

Stop revokes capture across the managed listener and native host. Received durable
inbox data remains and drains when enabled again. Companion events from an older
consent generation are discarded with a counter; they cannot cross Stop/re-enable
or a different catalogue. Older queued userscript samples are also rejected after Stop/resume; continuing legacy counters start from a new baseline, so stopped time is not added later. The app can finish the current bounded transaction on close.

“Allow companion background capture” separately permits the native host to capture
while Edge is running and Omoi is closed. It does not install a logon task, launch
Edge, or keep the HTTP listener alive after the app closes. The host exits on EOF or
60 seconds without messages; the companion disconnects an unused port after 30
seconds. Capture stops when the browser/companion stops. The advanced `omoi listen`
command still owns its own lifetime; app Stop is enforced when setup metadata exists.

## Development Edge companion

Use a disposable Edge profile for acceptance. End-user store distribution is pending.

1. Open Edge's Extensions page, enable Developer mode explicitly, choose Load
   unpacked, and select the package's `extension` directory. This is packaged
   content-script code, not a userscript manager. Note its exact extension ID.
2. Run the installer again and enter that ID on the optional companion page. The
   installer registers only a per-user native host. It does not install an extension,
   edit an Edge profile, grant permissions, or use force-install policy.
3. For a portable/custom-home installation, run the explicit registration action
   from that package directory (PowerShell):

   ```powershell
   .\Register-OmoiHost.ps1 -ExtensionId '<exact 32-letter ID>' -CatalogueHome 'C:\absolute\synthetic-home'
   ```

   Omit `-CatalogueHome` for the normal `%LOCALAPPDATA%\omoi` home. Keep a custom
   app `OMOI_HOME` and this registered home identical. A second home needs a separate
   package directory and explicit registration; queues never silently switch homes.
4. Open Omoi, enable capture, then open the companion popup. Choose Approve supported
   sites and approve the browser prompt. Denial leaves setup incomplete and can be
   retried. Reload existing video tabs, then choose Connect to local Omoi.
5. Play a supported video and return to Omoi. Verify a received timestamp, then a
   recorded timestamp and catalogue entry. A connection alone is not recording.

No token/port entry is used on this companion path. The extension cannot read,
export or delete catalogue data. Playback capture is limited to the seven explicit
site origins. Cookies, browser history and page-world messages are not requested.
Provider/ad/DRM coverage and Edge permission UI still need the Windows/device matrix.

## Userscript fallback

Use the in-app fallback instructions. Export script 0.2.0; import it through the
manager dashboard if opening the file displays raw code. Approve supported sites
and Allow user scripts if requested. From a foreground tab, use Copy local pairing
credential in Omoi, then Pair Omoi in the manager. The copy is an explicit local
clipboard action; do not share or paste it into a URL. The manager still asks for
the listener port. If dialogs were suppressed, retry in the foreground; a browser
restart is a recovery option, not proof of setup.

App updates do not update a manager's installed script. Reimport the exported
version to upgrade. Compatible 0.1.1 event ingestion remains available. The current
companion supports client 0.2.x (minimum 0.2.0) and protocol 1; incompatible clients receive an
upgrade action. Native queues keep event IDs over worker restarts and compatible
app upgrades. Userscript cross-tab manager storage still has its historical race;
the companion has one serialized, durable queue owner.

## Repair, updates, removal

Close Omoi before updating. Install/reinstall over the application directory; the
catalogue, config, consent, source scope, WebView2 appearance and listener credential
remain under the data home. Omoi has no automatic desktop download/updater. Install
only a build whose source/trust you have checked; unsigned development files may
trigger Windows warnings. Never automatically run an older binary against a newer
schema. Recovery uses a compatible binary or an explicitly selected pre-upgrade
backup, not an implicit downgrade.

Rerunning registration repairs an old executable path only when its Omoi ownership
marker is readable and its catalogue/channel match. Unknown ownership fails closed.
Moving a portable folder without a readable old marker needs explicit removal from
the old installation first. A different ID can be registered only by this explicit
action; development and release names are distinct (`org.omoi.capture_dev` and
`org.omoi.capture`). Production requires the owner to build the release companion
and select its actual store ID; no wildcard origin is supported.

To remove a portable registration:

```powershell
.\Register-OmoiHost.ps1 -Action Remove
```

Installer uninstall removes only its own native registration and its own collection
task executable path; another installation's registration/task is preserved. Data
and settings are preserved by default. Remove the browser companion through Edge's
normal extension UI. “Disconnect and clear unsent events” clears that companion's
local queue, not the recorded catalogue. Portable users should disable automatic
collection in Sources before deleting application files.

## Build and release preparation

```text
cargo build --locked --release --bins --example synthetic
python scripts/verify_desktop.py --bin-dir target/release
ISCC /DBinDir=<absolute release directory> packaging/omoi.iss
python scripts/verify_installer.py --installer dist/Omoi-Setup-unsigned.exe
python scripts/package_windows.py --bin-dir target/release --out dist/Omoi-portable-unsigned.zip
```

The last four commands require Windows, WebView2 and Inno Setup 6. The portable
packager rejects uncommitted tracked changes and verifies extracted executables.
CI runs synthetic installer/registry tests in a disposable Windows runner. Those
results must be read separately from Linux logic/stdio results.

Owner release actions: approve a real Edge store identity, build/register that exact
release identity, complete clean-device Edge and physical sleep/logon/battery tests,
measure the entire Windows GUI/browser/host process tree, then separately arrange
signing/store submission/public distribution. This task neither publishes nor signs.

## Playback-state upgrade and older queued Twitch events

The state-review follow-up upgrades the catalogue to schema 4. Stop all older Omoi processes and capture writers and back up the whole catalogue before first writable launch; schema-3 binaries cannot reopen the upgraded database. The accepted frontier intentionally survives item deletion so queued samples cannot resurrect that broadcast across a batch boundary.

Old queued Twitch events lack their original timezone. Ambiguous cases are preserved with an actionable error. Use the bounded read-only preview and explicitly selected event-ID migration described in [playback state recovery](state-audit.md); choose separate offsets for DST/travel groups, and do not guess when history is uncertain. No automatic queue reinterpretation is performed.
