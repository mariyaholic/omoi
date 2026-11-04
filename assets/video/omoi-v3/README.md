# Omoi video v3

[Watch or download the video](omoi-v3.mp4) · [Storyboard](storyboard.md) · [Contact sheet](storyboard.jpg) · [QA](QA.md)

40 seconds, 1920 × 1080, 30 fps, H.264 with 48 kHz stereo AAC. Real omoi captures, re-shot so that nothing is upscaled, short action captions, and pixel-art bookends.

## Contents

- `omoi-v3.mp4`: the viewing copy, 4,186,870 bytes
- `source/render.py`: picture renderer and film build, including the score
- `source/audio/`: the score, treatment A (Night Carriage) re-timed to this cut
- `source/capture.py`: rebuilds the synthetic sandbox and re-shoots every capture
- `source/validate.py`: measures the final file and writes `qa/validation.json` and `qa/copy-review.json`
- `source/capture-provenance.json`: commit, method, viewport, DSF, pixel size and SHA-256 per shot
- `capture/`: 20 genuine app screenshots
- `storyboard.md`, `storyboard.jpg`, `QA.md`, `qa/`: sequence and verification
- `requirements.txt`, `source/render-environment.json`: dependencies and recorded environment
- `LICENSES.md`: sources, and the SoundFont notice to keep with the film

Video SHA-256: `7a40b946afce3353a3df56e9b316dec4cd6245b6f8e642e1ab32d1ad9c992b67`

## Rebuild

Needs Python 3.13, FFmpeg and FluidSynth on PATH, the DejaVu Sans Mono font and the FluidR3_GM SoundFont at the paths in `source/render.py` and `source/audio/common.py`. From this directory:

```sh
python3 -m pip install -r requirements.txt
python3 source/render.py
python3 source/validate.py
```

The renderer writes the score and silent picture to `build/`, which is not part of the package, then `omoi-v3.mp4`, `storyboard.jpg` and `qa/camera.json`. `validate.py` needs `build/`, so run the renderer first. Exact hashes need the same tool versions.

To re-shoot, run `python3 source/capture.py --binary PATH/TO/omoi` with Playwright's Chromium installed. It defaults to `../../target/release/omoi` from this directory, and it deletes and recreates `../sandbox`.

## Capture boundaries

The screenshots come from the real connected browser UI of omoi, built from commit `cc530f0af90128c7ff933f80d782d7cfc2a31cf4` on Linux and driven with Playwright. They show eight synthetic titles and eight synthetic browser openings, imported by `omoi collect` from two fake browser profiles in a sandbox. No personal browser history, credentials or real browser profile was read.

This is a film made from genuine screenshots and camera moves. It is not a continuous screen recording and does not show the Windows app window. The tag editor is unsaved and labeled that way. The app's statement that tab time is not measured playback stays on screen. Audio was measured, not heard.
