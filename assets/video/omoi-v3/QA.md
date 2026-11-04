# Omoi v3: verification

Measured values are in `qa/validation.json`, `qa/camera.json` and `qa/copy-review.json`.

**Not done:** nobody has heard the audio, and nobody has watched the film play at speed. Decoded stills were inspected, not motion.

## Content

- The captures are genuine screenshots of the connected browser UI, built from the pinned commit. They are not illustrative UI or a continuous recording.
- 8 synthetic items and 8 synthetic openings came from `omoi collect` on two fake profiles in a sandbox. The capture script stops unless `omoi paths` reports only sandbox locations.
- The metadata save was not performed. After capture the catalogue still has 0 tags and every item is type `video`. The editor scene is labeled OMOI / UNSAVED PREVIEW.
- No page request left 127.0.0.1, and no JavaScript errors occurred. Settings captures are clipped above the section that displays filesystem paths.
- Authored on-screen text has no periods. The ending line comes from the README's first sentence, with one word removed and nothing added.
- The tab-time caveat is held on screen from 15.5 to 18 s. APP CAPTURE and SYNTHETIC DATA stay visible on every app scene, and the disclosure is on the end card.

## Picture

- 1920×1080, 30 fps, 1200 frames, BT.709 tags. Full decode returned exit code 0 with no errors.
- Largest camera scale over every rendered frame is 0.866, so nothing is upscaled.
- The largest frame-to-frame change in mean luma is 17.55 levels, at 34.533 s. No frame changes by more than 20.
- Hard cuts detected at 3, 7, 11, 13, 18, 21, 22.5 and 26 s. The detector also flags 14.8 s, which is the fast camera move, not a cut.
- Frame sheets from the final file were inspected at 6 frames per film second for 7–9, 11–13, 13–21, 21–29 and 29–35 s, plus full-size frames at 8.5, 17, 24 and 37 s. Phone-size legibility was not tested.
- Two capture runs gave identical hashes for all 20 PNGs, and renders after each run gave the same mp4 hash.

## Sound

Treatment A (Night Carriage), re-timed to this cut. Measured on the final mp4 with ffmpeg `ebur128=peak=true`:

| Measure | Value |
|---|---|
| Integrated loudness | −14.1 LUFS |
| Loudness range | 5.9 LU |
| True peak | −2.3 dBTP |
| Sample rate | 48 kHz, stereo AAC |
| Audio / video / container duration | 40.000 s each |
| A/V offset against the mix | 0 samples |
| Mono fold-down | 1.4 LU lower than stereo |
| Limiter, maximum | 7.43 dB, active 12.1% of the film |
| Spectral change, median | 7.7 dB at hits, 4.19 dB at non-cut seconds |
| Piano onset against hits | −3 to +7 ms, on the stem |

Listening priorities are in the project's V3 report: 28.9–34.5 s for limiter pumping, then the 31 s and 22.5 s hits.
