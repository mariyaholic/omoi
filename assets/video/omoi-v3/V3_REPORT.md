# Omoi film v3: report for the Manager

**All seven fixes (a–g) are done and the delivery specs are met.** Nobody has heard the sound, and nobody has watched the film play at speed. I checked decoded stills from the final file, not motion, and I can't hear audio. Those two checks need a person; see section 6.

- **File:** `omoi-v3/omoi-v3.mp4`, 4,186,870 bytes (v2 was 2,493,176)
- **SHA-256:** `7a40b946afce3353a3df56e9b316dec4cd6245b6f8e642e1ab32d1ad9c992b67`
- **Reproducible:** two capture runs gave identical hashes for all 20 PNGs, and renders after each run gave the same mp4 hash.
- **Measurements:** `omoi-v3/qa/validation.json` and `evidence/`.

## 1. Final file

Measured on the muxed mp4. Loudness and true peak use ffmpeg `ebur128=peak=true`.

| Measure | Target | v2 | v3 |
|---|---|---|---|
| Integrated loudness | −14 ±0.5 LUFS | −16.0 | **−14.1** (pyloudnorm on decoded audio: −14.14) |
| True peak | ≤ −2.0 dBTP | −1.5 | **−2.3** |
| Loudness range | none set | 1.0 LU | **5.9 LU** |
| Sample rate | 48 kHz | 96 kHz | **48 kHz** |
| Audio codec | AAC stereo | AAC | **AAC stereo**, 192.96 kb/s |
| Audio / video / container duration | 40.000 s | 40.000 s each | **40.000 s each** |
| Decoded audio samples | 1,920,000 | not measured | **1,920,000** |
| Picture | 1920×1080, 30 fps | same | **1920×1080, 30 fps, 1200 frames**, tagged BT.709 |
| Full decode | no errors | no errors | **exit code 0, no errors** |

Audio against `mix.wav` has 0 samples of offset. The AAC in the mp4 is identical to the encoded score, so muxing changed nothing. The first and last 5 ms peak at −62.0 and −64.7 dB.

## 2. What changed, item by item

### a. Capture resolution
- **Method:** Playwright drives the real UI in Chromium 156, on the synthetic catalogue. There are now 20 captures (10 in v2) at DSF 3 or 5, up to 5750×4855 pixels. v2 was about 1165×747 at 1×.
- **Result:** the largest scale in the film is **0.866, so nothing is upscaled**. v2's worst was 4.85× (table in section 4).
- **Sharpness at matching shots:**

| Shot (v2 s / v3 s) | Variance of Laplacian, v2 → v3 | Ratio | p99 gradient, v2 → v3 | Ratio | Mean gradient ratio |
|---|---|---|---|---|---|
| Search box (8 / 7.74) | 445 → 1479 | 3.32 | 83.8 → 101.2 | 1.21 | 0.81 |
| Result title (12 / 12) | 97 → 511 | 5.28 | 58.5 → 84.7 | 1.45 | 1.25 |
| Details values (15 / 14) | 5 → 114 | 22.95 | 20.3 → 34.7 | 1.71 | 1.07 |
| Openings caveat (19 / 17) | 36 → 497 | 13.72 | 38.8 → 72.6 | 1.87 | 0.89 |
| Tag editor (23 / 20) | 31 → 390 | 12.70 | 44.1 → 80.0 | 1.81 | 0.77 |
| Sources dialog (27 / 24) | 161 → 574 | 3.57 | 62.0 → 79.3 | 1.28 | 0.81 |
| Themes (30.5 / 27.5) | 321 → 844 | 2.63 | 71.5 → 85.0 | 1.19 | 0.89 |
| **Control:** same pixel art (1.8 / 1.8) | 435 → 414 | 0.95 | 71.7 → 71.7 | 1.00 | 0.98 |

- **How to read it:** the control barely moves, so the gains aren't an encoder effect. Framing differs between the films, so these ratios are not pure sharpness. Mean gradient falls below 1 on five shots. My likely explanation is v2's blur halos and different amounts of flat area, but I didn't test that. Judge by the crops: `evidence/compare-*.png` show v2 above v3 at 1:1.

### b. Ending
From 34.5 s the end card reads:
- `omoi`
- `Local watch history for the PC` (from 35.3 s)
- `Windows · development preview` (from 35.9 s)
- `APP CAPTURE / SYNTHETIC DATA`

The second line is the README's "Local, low-overhead watch history for the PC" with "low-overhead" removed. It adds no claim.

### c. Light-theme transition
The real Confluence page now dissolves in inside the dark stage (28.6–29.2 s). The stage then dissolves to paper (30.7–31.3 s). The full-frame light therefore arrives at about 31 s, not 32 s.

| Mean luma change between frames | v2 | v3 |
|---|---|---|
| Into the light | **189.21** in one frame at 32 s | 10.23 max inside the stage, 6.93 max for the stage |
| Out of the light | **199.13** in one frame at 35 s | 17.55 max, dissolve 34.2–34.8 s |
| Frames changing by over 20 levels, whole film | 2 | **0** |

The hard cut out of the light at 35 s wasn't on the fix list. It was the larger jump, so I fixed it too.

### d. Search filtering
- **7.0 s:** real empty search box.
- **7.13–7.73 s:** five real typed states, t-r-a-i-n, each landing within one frame of its piano note.
- **7.75–8.4 s:** the camera pulls back to show the list.
- **8.5 s:** Enter was really submitted in the capture. The real result takes over, and the list goes from "8 items" to "1 items".

The search box no longer jumps when the list filters. omoi centres short pages, so I used a 530 px window height, where the box stays at the same position (y 109.8) before and after.

See `evidence/frames-07-09s.jpg`.

### e. Sources dialog
The crop is now x 257.5–1111, y 150–496 CSS px, chosen from DOM measurements:
- Both profile rows (y 161.6–348.6) are inside the frame.
- The dialog (y 216.0–484.0) is whole.
- The crop stops above the Automatic collection box (y 503.7), so that box no longer shows at the bottom edge.

This needed a 700 px window height (747 px in v2). A new 1.5 s beat before the dialog ticks the two demo profiles, going from "0 selected" to "2 selected". Text the dialog itself covers, such as "Demo C…", is how the real modal looks.

### f. Find close-up
At 12 s, the rows holding content now span 72.7% of the frame height (46.3% in v2), and non-background pixels rose from 17.16% to 27.87%. The crop drops "Youtube / Opened · chrome", which is still readable in the 8.5–11 s shot.

### g. The three detail panels
- **Length:** the panels took 12 s in v2 (13–25 s) and take 8 s now (13–21 s).
- **Details and openings:** these are now one shot. One real sheet, one 0.9 s camera move, ending on the caveat as the caption and chord change at 15.5 s.
- **Longest static hold:** 4 s in v2, 2.5 s now. The Tag scene also has real typed states at 18.6 and 19.4 s.
- **Where the time went:** see the v2 column in section 3.

## 3. Cut list

| Time (s) | Caption | Picture | v2 length → v3 length |
|---|---|---|---|
| 0–3 | Find it again | Pixel night train | 3 → 3 |
| 3–7 | Browse | History, camera travels all eight rows | 4 → 4 |
| 7–8.5 | Search | Typed t-r-a-i-n, camera pulls back | 2 → 1.5 |
| 8.5–11 | Find | Enter: list filters to one, camera moves in | Find 4 → 4.5 (with next row) |
| 11–13 | Find | Close-up of the result | |
| 13–15.5 | Inspect | Unknown / 0m 0s / 1; move starts at 14.6 | 4 → 2.5 |
| 15.5–18 | Check openings | Tab-time caveat held 2.5 s | 4 → 2.5 |
| 18–21 | Tag | Tags typed; UNSAVED PREVIEW label | 4 → 3 |
| 21–22.5 | Choose sources | Two profiles ticked | Sources 4 → 5 (with next row) |
| 22.5–26 | Choose sources | Confirmation dialog | |
| 26–28.9 | Switch themes | Five themes, dark | Themes 3 → 5 (with next row) |
| 28.9–31 | Switch themes | Confluence selected, inside the dark stage | |
| 31–34.5 | Keep browsing | Stage goes to paper; catalogue in Confluence | 3 → 3.5 |
| 34.5–40 | omoi | Pixel train, end card | 5 → 5.5 |

## 4. Scale per shot

Output pixels per capture pixel, logged on every rendered frame. Above 1.0 would be an upscale. v2 figures are worked out from the crop boxes and zooms in its `render.py`, not measured from video.

| Scene | Capture | DSF | v3 scale, min–max | v2 max |
|---|---|---|---|---|
| Browse | `03-history` | 3 | 0.538–0.646 | 1.471 |
| Search | `04-search-*` | 3 | 0.545–0.698 | 2.041 |
| Find, wide | `05-search-result` | 3 | 0.545–0.698 | 1.795 |
| Find, close | `05-search-result-close` | 5 | 0.821–0.848 | 2.662 |
| Inspect, Check openings | `06-details` | 5 | 0.565–0.866 | 4.848 / 4.183 |
| Tag | `08-edit-tags-*` | 5 | 0.866 | 4.183 |
| Choose sources | `01-sources-*`, `02-confirmation` | 3 | 0.705 | 2.003 (dialog) |
| Switch themes | `09-appearance`, `09-appearance-confluence` | 3 | 0.646 | 1.949 |
| Keep browsing | `10-confluence` | 3 | 0.524 | 1.486 |

Viewport, DSF, pixel size and SHA-256 for every capture are in `omoi-v3/source/capture-provenance.json`.

## 5. Privacy and sound

**Privacy**
- Every omoi process ran with HOME, XDG_DATA_HOME, XDG_CONFIG_HOME and OMOI_HOME inside `work/sandbox`. The script stops unless `omoi paths` reports only sandbox locations (10 checked).
- `omoi collect` read two fake Chromium history files and imported 8 items and 8 openings.
- The tag editor was never saved: the catalogue still has 0 tags and every item is still type `video`. The capture script stops if the catalogue changes.
- No request left 127.0.0.1, and no JavaScript errors occurred.
- Settings captures are clipped above the Catalogue section, so no filesystem path appears in any image.

**Sound: treatment A, Night Carriage, the audio lab's recommendation**
- **Copy:** `audio/` holds the copy; `audio-lab` is untouched. The package carries the same two files in `source/audio/`.
- **Beyond re-timing, I also changed:**
  - raised some piano velocities at 13, 15.5 and 18 s
  - added a Gm/Bb chord at 28.9 s
  - split the Sources chord at 22.5 s
  - moved the mastering to a 192 kb/s AAC loop with `-ar 48000`
- **Cut response:** median spectral change is 7.7 dB at hits and 4.19 dB at 30 non-cut seconds.
- **Piano onsets:** within −3 to +7 ms of all 13 hits, measured on the stem.
- **Scene levels:** build −11.7, light −10.5, title −17.3.

## 6. What a person must check

**Listen (nobody has)**
1. **28.9–34.5 s, the priority.** The limiter peaks at 7.43 dB of gain reduction, more than the lab version's 6.24 dB, and is active 12.1% of the film. Listen for pumping or a squashed swell.
2. **31 s:** does the light land hard enough? Spectral change there is 6.31 dB, among the smaller hits.
3. **22.5 s:** the weakest hit, at 3.42 dB, because the chord barely changes. Is the dialog's entrance marked enough?
4. **Mono:** it loses 1.4 LU. Try a phone speaker.
5. **Overall:** does it suit the film at all? Do the five notes feel like typing?

**Watch at full speed**
1. **Camera moves:** 3.5–6.85 s, 7.75–8.4 s, 9.1–10.2 s, 14.6–15.5 s and 31.3–34.4 s. The 14.6–15.5 s move is fast, and the scene detector flags it at 14.8 s as if it were a cut.
2. **Sidebar shimmer:** its fine dither pattern may shimmer during Browse and Keep browsing.
3. **Dissolves:** captions and screens overlap for a few frames around 31 s. Is that acceptable?
4. **Reading time:** rows hold 1.6 s and the caveat 2.5 s. Both are short by design, and I only checked them in stills.
5. **Phone:** I didn't test legibility at phone size. The smallest text is in Browse and the Sources list.

**Decide**
1. **Fonts differ from v2.** The Mariyaholic-98 theme renders in JetBrains Mono here, the first installed font in omoi's own list, where v2 looks like a different monospace. Confluence renders in Tinos, Arimo and Cousine. A Windows PC may show different fonts.
2. **Windows on the end card, Linux in the capture.** The end card says Windows as you asked, but the screens are the browser UI from a Linux build at the same commit, not the Windows window. As in v2, the film doesn't show the Windows app window.
3. **Four invented titles.** v2's captures show only four of the eight. Browse now shows all eight, so I wrote the other four, listed in the provenance file. Please check you're happy with them.
4. **Actions not shown on screen.** Collect is never pressed (the dialog was cancelled; the data came from `omoi collect`). No pointer appears, so the theme click is implied by the dissolve. Each state is real, but the film is not a continuous recording.
5. **"1 items"** is the app's own wording, left unedited. It is on screen from 8.5 to 11 s.

**Small changes you didn't ask for**
- Captions are set by ink width, which fixes gaps like "B rowse".
- Stars no longer sit next to the copy, because in v2 one read as a period after "again".
- `SYNTHETIC DATA` is right-aligned.
- v2's pointer and scrollbars are gone.
- The file grew by about 1.7 MB to hold the sharper text.

## 7. Limits

- **Rebuilding:** `render.py` uses system paths for the DejaVu font and the FluidR3 SoundFont, as v2 did for its font. Everything else is relative. Exact hashes need the same tool versions, recorded in `source/render-environment.json`.
- **Scratch still on disk:** the delete was denied, so `omoi-v3/build/` (74 MB), `tmp/` (4.3 MB) and three `__pycache__` folders remain. They are not deliverables and can be deleted. `omoi-v3/.gitignore` covers `build/` and `__pycache__/`. `validate.py` needs `build/`, so if it has been deleted, run `render.py` first.
- **Licence:** the piano comes from FluidR3_GM, so its MIT notice ships in `omoi-v3/LICENSES.md`. That notice carries the copyright holder's name, as the licence requires.

## 8. Evidence

| File | Contents |
|---|---|
| `evidence/compare-*.png` | 7 shots, v2 above v3, same on-screen region and size at 1:1 |
| `evidence/sharpness.json` | Laplacian and gradient figures for the text region and the whole stage window |
| `evidence/frames-07-09s.jpg` and `frames-11-13s.jpg`, `frames-13-21s.jpg`, `frames-21-29s.jpg`, `frames-29-35s.jpg` | Grabs from the final mp4 at 6 frames per film second |
| `evidence/transitions.json`, `closeup-fill.json`, `v2-audio.json`, `v2-scale-factors.json`, `earlier-versions.json` | The other before/after numbers |
| `evidence/build_evidence.py`, `measure_earlier_versions.py` | Scripts that regenerate them |

The frame sheets extend past the requested ranges where the v3 scene runs on: 21–29 s covers 25–29 s, and 29–35 s covers 31–34 s.
