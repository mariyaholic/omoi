# Playback capture

Capture is optional. Ordinary history imports continue to work without it.

1. Run **Run Playback.cmd** (or `omoi listen`). Keep the console open. The listener uses the `listener_port` in config.toml, default 43821.
2. Install `omoi.user.js` in a userscript manager supporting the Tampermonkey GM APIs. Review its platform permissions.
3. On a supported video page, open the manager's **Omoi playback → Pair Omoi** command. Paste the contents of `%LOCALAPPDATA%\omoi\listener\token`, then enter the listener port. A custom OMOI_HOME changes that path. The token is stored in the manager, never in page HTML or a URL.
4. Play a video. In Omoi Settings, choose **Process playback**, or run `omoi collect`. Collection consumes the event inbox transactionally. Use **Check playback inbox** to inspect pending/dropped counts.
5. To stop, close the listener console and use **Stop capture and clear pending events** in the userscript menu. Existing sessions remain in the catalogue.

The script observes HTML video elements on YouTube, Netflix, Twitch, Bilibili, Prime Video, Disney+ and Crunchyroll. This generic adapter is not proof of compatibility with every current player, DRM flow, advertisement or userscript manager. Actual manager transport and platform playback require manual validation. Page Origins are rejected by the listener; a manager that adds a page or extension Origin is not supported by this version. Do not weaken Origin checks to work around this.

The listener binds loopback only and accepts authenticated event writes; it cannot query/export/delete the catalogue. The token directory is protected for the current Windows user (0700 directory/0600 token on Linux). Delete the token file while the listener is stopped to rotate it, then pair again.

Metadata observed during playback fills missing title, channel and duration fields. Manual edits take precedence. No platform metadata API or paid service is called. Seek jumps and gaps longer than five minutes are not counted as playback. Overlapping captures for one item use the highest measured interval density rather than adding tabs; sub-interval timing remains approximate. There is no Windows SMTC capture in this build.

Queues persist across reloads in manager storage: at most 32 queues of 256 events, retry backoff up to 60 seconds. A full queue drops its oldest event; excess new queues are refused until older queues drain. The listener inbox caps at 100,000 events and 20 MB of event bodies. Processed receipts persist for replay protection; database size can grow with retained usage. No automatic capture task is installed.

API reference: [Tampermonkey documentation](https://www.tampermonkey.net/documentation.php?locale=en).
