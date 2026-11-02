<!-- Copy of ../STYLE.md in the mariyaholic repo (the estate style guide). Edit the original, then recopy. -->

# Style guide: documentation and public promotion

Applies to READMEs, ARCHITECTURE and CHANGELOG files, the catalogue and site, release notes, keynote films, and anything posted about a product.
Date: 2026-10-28. Owner: Maria. Status: draft 1. Written from the existing material and not yet applied to it (see section 11).

## How this was made

A read-only pass over the READMEs and CHANGELOGs of `latch`, `loupe`, `kotonoha`, `shijima`, `sashigane`, `warifu` and `mh-ticker`; `catalog.json`, `catalog.md` and `site/index.html`; the `promo-v3` and `promo-v4` plans and each crate's `promo/evidence.json`; `design/uika-canonical.css`; the Omoi docs; and the estate `copy-craft` skill.

Unmarked rules are things the existing material already does, written down. Rules marked **(new)** fill a gap with no precedent and are proposals for Maria to accept or change.

This guide is not estate law (that is `AGENTS.md`) and not a token sheet (that is `design/uika-canonical.css`, never hand-edited here).

---

## 1. Voice

One voice for every surface. Tone changes by surface (section 2); the voice does not.

**Traits**

1. **Plain.** Says what the thing does in words a Windows user knows. The API name comes after the plain sentence.
2. **Honest about limits.** States what is not built, not measured or not safe before the reader finds out.
3. **Specific.** A number, path, command or named API in place of an adjective.
4. **Personal, quietly.** First person singular where the choice is Maria's ("Everything I build, kept in one room"). A byline. No mascot voice in docs.

**Rules**

1. Lead with what it does, then its status, then how.
2. Put what is missing next to what exists. Write "What it does today", not "Features".
3. An adjective that implies a comparison (lightweight, fast, small) gets its number and its conditions, or it goes.
4. Present tense for shipped behaviour. "Planned for M3" or "will" for everything else. Never present tense for something unbuilt.
5. Before a risky command, say what it does to the user's machine.

**Do-words:** measured, today, planned, only, by default, undo, source.

**Don't-words:** ultra-lightweight, uncompromising, sleek, blazing, seamless, powerful, robust, cutting-edge. "Zero" is allowed only as a verifiable fact about a dependency list or a count ("0 plaintext `.env` files on disk"), never as a mood ("zero bloat").

**Five real examples to copy**

- Latch README: "Measured on the author's machine: ~4 MB of RAM working set and ~0.0% idle CPU (anecdotal, not a benchmark)."
- Loupe README: a section titled "What Loupe actually measures (read this first)" that says HTML boxes are estimated, not laid out.
- Latch README: "Dry-runs are safe. `unlock --dry-run` and `env clean --dry-run` query and print holders + plan, then kill nothing and write nothing."
- Kotonoha README: a `TODO(screenshot)` comment saying no screenshot is embedded "rather than shipping a mock-up".
- Omoi `ARCHITECTURE.md`: "Status: design only. M0 U1 partial. Does not build yet: `src/lib.rs` declares `pub mod readers;` and `src/readers/mod.rs` is missing."

**Two rewrites of existing text**

Before (Latch README, opening):
> A native, ultra-lightweight Windows system lock breaker, port unlatcher, live environment & `PATH` broadcaster, startup sentry, and zero-plaintext DPAPI secret runner. Zero Electron, zero webview, zero bloated runtimes.

After:
> Latch is a Windows tray app and command-line tool. It shows which process holds a locked file or a busy port and can end it, checks your `PATH`, lists startup entries, and runs commands with secrets from Credential Manager. Rust and Win32; no Electron or webview. About 4 MB of RAM on the author's machine (one machine, not a benchmark).

Before (Latch README, feature 1):
> Tired of Windows crying *"The action can't be completed because the file is open in another program"* (`os error 32`)? Latch queries the Windows Restart Manager API, pinpoints the holding PID, process name, and executable path, and severs the lock cleanly

After:
> When Windows says "The action can't be completed because the file is open in another program" (`os error 32`), Latch asks the Restart Manager which process holds the file and shows its PID, name and path. It can then close that process: a polite `WM_CLOSE` first, `TerminateProcess` if it is still alive.

---

## 2. Tone by surface

Tone uses the four NN/g spectrums as vocabulary: formal↔casual, serious↔funny, respectful↔irreverent, matter-of-fact↔enthusiastic. These are labels for choosing, not a measured result.

| Surface | Tone | Notes |
|---|---|---|
| README | Casual, matter-of-fact | Whole product understood from the first screen. Structure in section 4. |
| ARCHITECTURE and design docs | Formal, serious, matter-of-fact | Status line and date at the top, sources named, tables for modules and budgets, a decisions table with who and when. |
| CHANGELOG, release notes | Matter-of-fact, past tense | Keep a Changelog sections. Describe behaviour, not adjectives. State exit codes and defaults. "Documentation truth pass" entries are welcome. |
| Error and CLI messages **(new)** | Serious, brief | Say what happened, what was not changed, and what to do next. Latch's "prints the plan, writes nothing, exits 1" is the model. |
| Site and catalogue | Casual, warm, first person | The student-council-room metaphor lives in chapter taglines ("Tonight's agenda"). Each tool's one-line job stays plain. |
| Video on-screen text | Terse, matter-of-fact | Existing lines run three to five words ("Desktop stutters aren't random."). A sourced figure may run longer. |
| Posts and announcements **(new)** | Casual, matter-of-fact | Section 8. |

**Catalogue `job` lines (new cap):** at most 25 words, what it does for a person first, details in the README. Current lines run 13 to 32 words; three are over the cap (MH Ticker 27, Shijima 27, Kotonoha 32).

Example, Shijima (27 words today) compressed from its own list, dropping "sub-millisecond" because the catalogue gives no evidence for it:
> Windows audio and voice governor: headphone-unplug mute, Bluetooth 48 kHz lock, loudness limiter, push-to-talk, stream submix isolation.

---

## 3. Claims, numbers and status

### The evidence rule

A public number or comparison needs a source before it is published. Write the claim as a sentence, find the row that supports that sentence including its scope, and if there is no row, delete the claim. Do not rescue it with "up to" or "can".

- **Films:** every number comes from the crate's `promo/evidence.json` (`{id: {v, src}}`). The renderer reads figures with `fig()`, incidental digits are whitelisted in `_ui_tokens`, and every key must be used (`EV.unused() == []`).
- **READMEs:** a measured figure says what was measured, where, and how sure we are. Latch's "on the author's machine ... anecdotal, not a benchmark" is the pattern. **(new)** Add the date and the method when you can.
- **Budgets are not measurements.** Omoi's 30 MB, 10 MB and 100 MB figures are acceptance targets. Public copy calls them targets until a measurement exists. None exists today.
- **Comparisons** name both sides, measure both the same way and state the conditions. The Kirameki test (mpv against headless Edge, both measured headless and muted) is the model. Never say a competitor's product lacks something without checking.
- **Context travels with the number.** A true figure in the wrong context is a false claim.

### Number format

| Thing | Rule | Example |
|---|---|---|
| Unit | Space between number and unit | `4 MB`, `16 µs`, `240 Hz`, `−14 LUFS` |
| Percent | No space | `0.0%`, `78.4%` |
| Thousands | Comma | `2,110 chars`, `522,752 bytes` |
| KB and MB in public copy **(new)** | 1,024-based, as Windows Explorer shows them. Keep the exact byte count in the evidence file or contract | `510 KB` for 522,752 bytes |
| Precision | One per claim. Round for the claim, keep the exact value in the footnote | `~4 MB`, not `4.1 MB` beside `4 MB` |
| Percent vs points | Say which | "−1 percentage point", not "−1%" |
| Dates | ISO | `2026-10-28` |
| Small bases | Give the base | "3 of 12 products", not "25%" |

### Status words

Use the catalogue's four: **shipped**, **beta**, **planned**, **retired**. Anything with no working build is **planned**, including a finished design. Placeholder catalogue entries carry the label "sample entry". On the site a dashed border means not built yet; keep that meaning wherever status is drawn.

### Screenshots and mock-ups

Real captures only. Where none exists, leave `<!-- TODO(screenshot): ... -->` with a note on how to get one. A block of ASCII art that stands in for a screen must be labelled as such, as Latch's README does. A render that is not a capture says so in its caption.

---

## 4. Documentation structure

### Files per product

`README.md`, `ARCHITECTURE.md`, `CHANGELOG.md`, `LICENSE`; add `CONTRIBUTING.md` and `SECURITY.md` when the repo goes public; `ROADMAP.md` when there is one. Film material lives in `promo/` (`PLAN.md`, `STATUS.md`, `evidence.json`). `AGENTS.md`, `CLAUDE.md` and `WORKSTACK.md` are estate files, not public documentation.

### README order

1. `# Name`, then the byline (section 5).
2. A scope line in italics: platform · language · what it does not need. Shijima's is the model: "Windows 10/11 x64 only · pure Rust + Win32/WASAPI · no drivers, no network, no install".
3. One paragraph: what it is and who it is for.
4. Status and measured facts, with their caveat.
5. A screenshot, or `TODO(screenshot)`.
6. **What it does today.** A numbered list; each item is a plain sentence, then the API.
7. **Read this before you <risky verb>**, when anything is destructive. Include the dry-run.
8. Quickstart: commands in fenced blocks with the expected result.
9. Command reference and exit codes.
10. Limits and not-yet, linking `ROADMAP.md`.
11. Build and verify; layout.
12. Link to `ARCHITECTURE.md`.

### ARCHITECTURE order

Status line and date; the sources it was built from; what it is; process or module map; data flow; invariants; budgets (targets or measurements, labelled); a decisions table with date and who decided; open items; a "Resolved on <date>" list when documents were reconciled. The Omoi `ARCHITECTURE.md` follows this.

### Mechanics

- Headings in sentence case: "What it does today", not "What It Does". One `#` per file.
- Fenced code with a language tag. Windows commands in `powershell`. **(new)** One command per fence when it is meant to be run.
- Relative links inside a repo. No bare URLs in prose.
- Emoji only where they carry state: the tray dots 🟢🟡🔴 and a ⚠️ on a "read this first" heading. None in other headings or body text.
- Record a decision with its answer, who gave it and the date: "Maria, 2026-10-25".
- Handoffs between agents use the six-line block in `AGENTS.md`; this guide does not restate it.

---

## 5. Names and identity

- **Product names** are plain Japanese words tied to Uika's motifs (stars, night, first flower, stage). No franchise proper nouns, to keep public repos clear of trademarks. Ask Maria before creating any public repo.
- **One source for name, tagline and id:** `catalog.json`. READMEs, film evidence and the site read from it; a mismatch means one of them is stale.
- **Spelling:** `Latch` in prose, `Latch.exe` for the binary, `LATCH.EXE` only inside a PC-98 screen.
- **Chapters:** Quality of Life (`qol`), Looks (`look`), Presence (`pres`), Throughput (`thr`), Play (`play`), Plugins (`plug`). Ids run `mh-<chapter>-NNN`.
- **Byline:** `_made with love by [maria](https://github.com/mariyaholic)_` on its own line under the title. Two READMEs use it today; Shijima folds it into its scope line with a different link text.
- **Authorship:** no Claude or other AI authorship lines on commits or PRs in this repo (Maria's rule).

---

## 6. Visual identity on public surfaces

- **Tokens** come from `design/uika-canonical.css`. Bind roles, not literals; a new look is a parameter set over the same roles, never an edited copy.
- **Ground and accent:** dark by default (`--bg-deepest #060509`, `--bg-ground #0b0913`), one accent, Uika gold `#BB9955` (bright `#D8B878`). Text `#D8D4CE`, bright text `#E9E4DA`, muted `#97908F`.
- **Type:** Cinzel for display, Outfit for body, JetBrains Mono for coordinates, codes and tags. The site currently uses system fallbacks because the WOFF2 subsets are not self-hosted yet. No font CDN.
- **Status lamps:** shipped sage `#6E9C7E`, beta marigold `#E08D3C`, planned lavender `#A497D6`, retired lining red `#A2434D`.
- **Registers:** PC-98 owns the space, modern-moe owns the person, hand-drawn owns the annotations. One register per thing.
- **Page rules** (from the site and its audit): one static page, no framework, no trackers, nothing that leaves the page. Complete and readable under `prefers-reduced-motion`. No horizontal overflow at 375 and 1440 px. Visible focus. Text contrast at APCA Lc 60 or higher for body and 45 or higher for headings (Loupe's thresholds).
- **Omoi UI (new):** inherits the same canon. No separate palette.

---

## 7. Promotional films

### What is house style and what is per-film

**House style** (applies to every film):

- **Spec:** 20.0 s, 1920×1080, 30 fps, H.264 + AAC, loudness target −14 LUFS, true peak at or below −2 dBFS.
- **Look:** the v4 PC-98 look: 640×360 index canvas, 16 named colour slots, Bayer dither only, bitmap type, whole-pixel positions, hard cuts, CRT export. v4 replaced the v3 modern plates after Maria called the earlier films "extremely faulty and disappointing".
- **Time comes from the audio:** every cut lands on a measured event from `analyze_audio.py`, and reveals land on the beat grid.
- **Evidence lock:** section 3. The film's source line names the evidence file and the README.
- **One idea per beat.** Prefer a mechanism beat to a metric card.
- **Caveats are visible**, not trimmed. In Loupe's v4 film the caveat is the loudest beat.
- **A sample is labelled as a sample,** with its source line on screen.
- **Hero stat last** (number-last).
- **Language on screen:** English.
- **The brand end card closes every film:** gold ring, product glyph, name, role, chapter, a pill stat, a footnote.

**Per-film hands** (world, constraint, signature, forbid list) are drawn per film with `uika creative deck` and logged in that film's plan. They are not house rules. Do not copy a sibling's hand.

### Gates before a film is called done

1. `uika video qa --expect-duration 20` passes with 0 fail and 0 warn.
2. Frames are looked at on a contact sheet, not only measured.
3. A passing gate is not a quality claim. The v4 plan says so; keep saying it.

Deliverables per film: `<slug>_keynote_v4.mp4`, `_poster.png` and `_sheet.png`, mirrored into the crate.

---

## 8. Posts and short copy (new)

No post has been written under this guide, so these are proposals.

1. Line one: what it is and who it is for, in plain words.
2. Line two: one proof with its source, or one thing you can try.
3. Line three: one limit, stated plainly.
4. Link to the README, not the film.

No claim a README would not make. Nothing is posted under a repo or product name without Maria's yes.

---

## 9. Omoi voice file

Following `copy-craft`: one short file per product, with traits, rules, do-words, don't-words and real examples.

**Traits:** private, quiet, exact, modest.

**Rules**

1. Say where the data lives: one SQLite file under `%LOCALAPPDATA%\omoi\`.
2. Say what is not collected: the yt-dlp source is off, and phone and TV sources are out of scope.
3. Use "designed to", "planned" and "target" until the thing is built and measured.

**Do-words:** local, one file, off by default.

**Don't-words:** AI-powered, smart, seamless, "tracks you", cloud.

**Real examples (from the Omoi docs)**

- "Omoi is a local, PC-only watch-history app."
- "All data lives in one SQLite file under `%LOCALAPPDATA%\omoi\`. Nothing syncs to the cloud."
- "S4 is off and outside M0-M3."
- "No measurements exist yet."
- "Status: design only. M0 U1 partial. Does not build yet."

**What Omoi can say publicly today**

| Claim | Publishable now? | Condition |
|---|---|---|
| Local, PC-only, one SQLite file, no cloud sync | As design | Say "designed to" until M0 builds and runs |
| M0 makes no network requests | As design | Same condition |
| Memory, CPU and disk figures | No | Targets only. Call them targets, never "uses" |
| Hourly collector that exits | As design | Same condition |
| Recommendations | No | Scoring formula is unspecified in the docs |

---

## 10. Before you publish

- [ ] The first sentence says what it does; status and limits are stated.
- [ ] No don't-words. Every comparative adjective has its number.
- [ ] Every number has an evidence row or a measurement note (machine, date, method). Budgets are labelled targets.
- [ ] Name, tagline and id match `catalog.json`.
- [ ] Screenshots are real, or `TODO(screenshot)`.
- [ ] Dates ISO; units spaced; percent unspaced; thousands with commas.
- [ ] Destructive actions have a "read this first" section and a dry-run.
- [ ] Films: gates passed and frames viewed.
- [ ] Tokens from the canon; no CDN, no tracker.
- [ ] No AI authorship lines. Public repos and posts only after Maria's yes.

---

## 11. Drift pass, 2026-10-28

Fixed in the source files (local edits, nothing committed or pushed):

- **Latch id:** now `mh-thr-001` in the film's `evidence.json`, `intro.py`, plan and status, and in `promo-v4/render_v4.py`. Rendered videos still carry the old tag until re-cut.
- **Latch README:** opening rewritten without "ultra-lightweight", "sleek" or "zero bloated runtimes"; feature 1 rewritten; headings in sentence case.
- **Other READMEs:** byline added to Loupe, Kotonoha, Sashigane and Warifu, and split from the scope line in Shijima. MH Ticker lost "extremely resource efficient" and "any potato CPU won't feel it". Loupe and MH Ticker headings in sentence case.
- **Catalogue:** MH Ticker, Shijima and Kotonoha shortened to 25 words or fewer. Sashigane's "85% reduction" is now "85.7% smaller on the sample fixture", matching its README. `site/build.mjs` rebuilt `catalog.md` and `site/index.html`; `site/audit.py` passes.
- **Percent spacing:** `78.4%` in the v4 status file; no space before `%` in the Omoi architecture, feature and draft docs.
- **Loudness:** the Latch plan marks its −15 LUFS target as superseded by −14 LUFS and ≤ −2 dBFS.

Held, because the fix is a new film and not an edit (a HOLD notice sits at the top of each status file):

- **Latch films (v1, v3, v4):** show a monitor-topology vault, a DPC/ISR monitor, a 240 Hz pin, an EcoQoS governor and a standby-RAM purger, with figures for them. None of these appear in `latch/src` or in its README, ARCHITECTURE, ROADMAP or CHANGELOG, and the DPC trace is simulated. Re-brief from the README and catalogue.
- **Shijima films:** name "Focus Sanctuary, Circadian Warmth". No match for circadian, warmth or focus in `shijima/src` or its README.
- **Sashigane films:** state a 78.4% prune ratio. The README and ARCHITECTURE document 85.7% on the sample fixture; no source for 78.4% was found.

Not checked: the Loupe, Kotonoha and Warifu films beyond their subtitles. Check each on-screen figure against the code before publishing any film. Omoi docs still write budgets in exact bytes (fine for the contract); public copy converts them under the KB and MB rule.
