# Omoi film: learnings

Rule first, then what went wrong. "v1" is the two first attempts: `production` (26 s, drawn UI) and `rev1` (40 s, first real captures). "v2" is PR #7. Measured figures are in `evidence/earlier-versions.json`.

## Capture resolution
**Rule: work out the largest crop scale before capturing, and capture so that no crop in the film upscales. Fail the render if one does.**
- **What went wrong:** rev1 and v2 captured at 1× (about 1165×747), then blew crops up. v2's details panel reached 4.85×, so the text was blurry.
- **Why:** the screenshots came first and the crops were designed later. Nobody computed or logged the scale.
- **Now:** v3 captures at DSF 3–5 and logs scale per frame. Its largest is 0.866.

## Framing
**Rule: choose each crop from measured element positions, and make it fill the stage. Every edge lands on empty space or a page edge.**
- **What went wrong:** v2 cut the sources list mid-row, left the Find close-up mostly empty (content spanned 46.3% of the height), and left side columns on some panels.
- **Why:** crops were hand-picked numbers, checked on a contact sheet.
- **Also:** if the app shifts layout between states, change the window size until it holds still. Don't chase the shift with the camera.

## Captions
**Rule: captions are short action words with no authored periods. Check them in stills, including anything that can read as punctuation.**
- **What went wrong:** rev1 used full sentences ("You choose what comes in."). v2 fixed that, but a star beside "again" still read as a period, and fixed-width setting produced "B rowse".
- **Why:** the copy was reviewed as text, not as pixels on the frame.

## Endings
**Rule: the last frame must say what the product is, in one plain line from its README, plus its status and the disclosure.**
- **What went wrong:** v2 cut the closing tagline along with the slogans and ended on the brand alone, so a new viewer never learned what omoi is. `production` had carried a status line ("Windows app · Development preview").
- **Why:** "fewer words" was applied without asking what the ending has to do.
- **Also:** keep ending copy to the README's claims. Removing a word is fine; adding a claim isn't.

## Theme transitions
**Rule: no frame may jump more than 20 levels of mean luma. Measure it on the final file. Bring light in inside the dark stage first, and ease it out too.**
- **What went wrong:** v2 jumped 189.21 levels in one frame at 32 s and 199.13 at 35 s, and rev1 peaked at 197.21. The film then moved straight on. Only the first jump had been noticed.
- **Why:** contact sheets can't show the jump between two frames, and nothing measured it.
- **Now:** v3's largest is 17.55.

## Loudness and sample rate
**Rule: set a delivery spec first. Measure it on the final muxed file, not the WAV. Never use a filter that can change the sample rate, and force `-ar 48000`.**
- **What went wrong:** rev1 and v2 shipped 96 kHz audio, a likely side effect of `loudnorm`, at −16 LUFS with a 1.0 LU range. `production` was −1.5 dBTP, above today's −2.0 ceiling, and its audio ran 5 ms past its picture.
- **Why:** QA reported "about −16 / −1.5", the settings requested, rather than checking rate and range on the delivered file.
- **Now:** master in a loop on the AAC itself, mux it with `-c copy`, then measure the mp4 again. v3 is −14.1 LUFS, −2.3 dBTP, 5.9 LU, 48 kHz, with audio and video both 40.000 s.

## Sound to picture
**Rule: one cut list drives both picture and score, and the render fails if they disagree. Every hit gets a designed event. Check it against non-cut moments.**
- **What went wrong:** v2's score ran on an even 2 s grid and ignored the edit. Spectral change was 18.1 dB at cuts and 18.6 dB elsewhere. Its resolve came a second after the title, and it went silent before the end.
- **Why:** the score was written before the edit and never tied to it.
- **Now:** v3 measures 7.7 dB at hits and 4.19 dB elsewhere, and its typed frames land on their notes.
- **Also:** measure onsets by level. A flux peak read +47.5 ms on a note that really began within 10 ms.
- **Also:** sound must not claim more than the picture: no "saved" cue on an unsaved preview.

## Honesty in QA
**Rule: report only what was measured, and say who has not heard or watched it.**
- **What went wrong:** no version has had a listening pass. Scripts can't judge taste, pumping or motion, so each release has to name those checks for a person.
- **Also:** check the measuring tools too. v3's first evidence script reported the red channel as "luma". A disagreement with a second script exposed it, so keep two independent measures of anything important.
- **Also:** a control keeps comparisons honest. Identical pixel art in both films showed the sharpness gain wasn't an encoder effect.

## Privacy
**Rule: synthetic data only. Point every home and config variable at a fresh sandbox, and stop if the app reports any path outside it.**
- **Also:** clip captures above anything that displays a path. Check the catalogue is unchanged after capture, which is how the unsaved preview stays unsaved.
- **Status:** no version has had a privacy failure. Keep this rule anyway, because one mistake here can't be undone.
