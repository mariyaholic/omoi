# Omoi icon

The owner selected **4C — Pixel return** on 2026-11-03: a stepped open O with
a coral star in the upper-right gap. The O refers to Omoi and returning to watch
history. The star connects to the owner's existing pixel-star identity.

The production mark uses a 32-unit grid with even coordinates so its essential
edges and gaps also align at 16 pixels. The stepped background corners are
transparent outside the tile. Colours are indigo `#120c24`, cream `#f4cf8a` and
coral `#f08868`, from the Mariyaholic-98 palette. One mark is used across all
five interface themes. It has no animation, gradient or lettering.

`icon.svg` is the editable master. PNGs cover 16, 20, 24, 32, 40, 48, 64, 128,
256 and 512 pixels. `icon.ico` includes all sizes through 256. `icon-32.rgba`
provides 4,096 bytes for the native window without an image-decoder dependency.

Regenerate or verify exports with:

```text
uv run scripts/build_icons.py
uv run scripts/build_icons.py --check
```

The exporter requires Pillow 12.3.0 only during asset authoring; exports are
committed, so normal Rust builds do not need Python. Exports retain hard pixel
edges. The non-integral 20/24/40/48 sizes use nearest-neighbour fitting; the
primary 16/32 sizes have exact grid alignment.

The UI serves `/icon.svg` and `/favicon.ico`. The desktop window uses the RGBA
export; `build.rs` embeds the ICO in `omoi-desktop.exe` for Explorer and taskbar
identification. Windows GNU builds need `windres` on PATH (or `WINDRES` set);
MSVC builds need the SDK resource compiler `rc.exe` (or `RC` set). Other OS
builds skip the Windows resource step. The portable package also includes
`Omoi.ico` for explicit shortcut assignment.

The original generated comparison sheet was a concept reference. These assets
are a new vector drawing of the selected direction, with a more articulated
star and flat fills, rather than a crop of the generated sheet.
