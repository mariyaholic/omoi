# Omoi video v2

[Watch or download the video](omoi-v2.mp4) · [Storyboard](storyboard.md) · [Contact sheet](storyboard.jpg)

40 seconds, 1920 × 1080, 30 fps, H.264 with stereo AAC. This is the concise-copy revision: short action captions, no authored periods, real Omoi captures and pixel-art bookends.

## Contents

- `omoi-v2.mp4`: the exact delivered viewing copy, 2,493,176 bytes
- `source/render.py`: complete picture renderer and original synthesized score
- `capture/`: ten genuine app screenshots used by the renderer
- `source/capture-provenance.json`: source commit and capture hashes
- `storyboard.md`, `storyboard.jpg`, `QA.md`, `qa/`: sequence and verification
- `requirements.txt`, `source/render-environment.json`: render dependencies and recorded environment

Video SHA-256: `77b1e64baaa7d582645e5554480c144571e5b2edb12f548be0d15c300c688fdc`

## Rebuild

Use Python 3.12, FFmpeg on PATH, and DejaVu Sans Mono at the font path in `source/render.py`. From this directory:

```sh
python3 -m pip install -r requirements.txt
python3 source/render.py
```

The renderer creates the score, viewing copy, high-quality master and storyboard frames. The committed video is the viewing copy; the large intermediate video and WAV are deliberately omitted. Rendering is deterministic within a fixed dependency and encoder environment; other FFmpeg builds may produce different encoded hashes.

## Capture boundaries

Screenshots were captured from the real connected Rust browser UI built at commit `cc530f0af90128c7ff933f80d782d7cfc2a31cf4`. They use eight synthetic titles and eight synthetic browser openings from isolated demo profiles. No personal browser history, credentials, catalogue database, fixture database or browser profile is included.

This is a film made from genuine screenshots and camera crops, not a continuous screen recording or a capture of the Windows-native shell. The tag-editor preview is unsaved and labeled accordingly. The app's explanation that tab time is not measured playback remains visible.

The reference imagery and music are not reused. Reference availability and sources are documented in the storyboard. Both final exports passed complete decoding; audio was measured, not subjectively auditioned.
