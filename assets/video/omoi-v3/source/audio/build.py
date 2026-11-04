#!/usr/bin/env python
"""Treatment A, Night Carriage, re-timed to the v3 cut list: beatless pad and soft piano."""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import *  # noqa: E402,F403

rng = np.random.default_rng(101)

# One chord per scene; every harmonic change sits on a picture event. (start, end, voicing)
CHORDS = [
    (0, 3, [38, 50, 57, 60, 64, 65]),           # Dm9, night
    (3, 7, [38, 50, 57, 60, 65, 69]),           # Dm9, opened out
    (7, 8.5, [34, 46, 53, 57, 60, 62]),         # Bbmaj9, search
    (8.5, 11, [41, 53, 60, 64, 67, 69]),        # Fmaj9, the list filters: found
    (11, 13, [45, 53, 60, 67, 69, 72]),         # F/A, closer
    (13, 15.5, [31, 43, 50, 58, 65, 69]),       # Gm9, inspect
    (15.5, 18, [33, 45, 52, 60, 64, 67]),       # Am7, check openings
    (18, 21, [34, 46, 53, 57, 62, 65]),         # Bbmaj7, tag (kept unresolved: nothing is saved)
    (21, 22.5, [38, 50, 57, 60, 64, 65]),       # Dm9, the two ticks
    (22.5, 26, [38, 50, 57, 60, 65, 69]),       # Dm9, opened out: the dialog
    (26, 28.9, [31, 43, 50, 58, 65, 69]),       # Gm9, themes
    (28.9, 31, [34, 46, 53, 58, 65, 69]),       # Gm/Bb, Confluence inside the dark stage: the climb
    (31, 34.5, [36, 48, 58, 62, 65, 72, 77]),   # Bb/C (C11), the stage opens to light, suspended
    (34.5, 40, [29, 41, 53, 60, 64, 67, 69]),   # Fmaj9, title: the cadence lands
]

# Soft piano, (time, midi, velocity). Downbeats are exact picture events; the small figures mirror the action.
PIANO = [
    (0.45, 69, 38), (1.05, 72, 40), (1.65, 74, 42), (2.35, 69, 34),                 # "Find it again": the question
    (3, 50, 44), (3, 62, 50), (4, 69, 38), (5, 72, 36), (6, 76, 34),                # Browse
    (7, 46, 46), (7, 65, 50), (7.13, 74, 34), (7.27, 77, 34), (7.43, 81, 34), (7.57, 84, 34), (7.73, 86, 36),  # five keys: t-r-a-i-n, on the typed frames
    (8.5, 41, 58), (8.5, 69, 66), (8.5, 77, 72), (9.5, 72, 40),                     # Filtered: the first bright voicing
    (11, 81, 64), (11, 84, 56), (12, 79, 36),                                       # Closer crop: an octave nearer
    (13, 43, 52), (13, 62, 52), (13.8, 70, 38), (14.5, 69, 36),                     # Inspect, falling
    (15.5, 45, 50), (15.5, 64, 50), (16.4, 67, 36), (17.2, 72, 36),                 # Check openings
    (18, 46, 50), (18, 65, 52), (18.6, 69, 36), (19.4, 72, 36), (20.2, 74, 32),     # Tag: one note per typed tag, no cadence
    (21, 50, 46), (21.35, 62, 40), (21.35, 69, 46), (21.8, 62, 40), (21.8, 74, 46),  # Two ticks
    (22.5, 50, 54), (22.5, 69, 56), (24, 76, 40), (25, 77, 36),                     # The dialog
    (26, 43, 48), (26, 70, 44), (26.9, 74, 42),                                     # Themes
    (28.9, 46, 50), (28.9, 77, 54), (29.6, 81, 54), (30.3, 84, 58),                 # Confluence selected, rising
    (31, 36, 76), (31, 48, 76), (31, 74, 84), (31, 77, 88), (31, 84, 94), (32, 79, 54), (33, 81, 48),  # The light
    (34.5, 41, 62), (34.5, 53, 56), (35.3, 69, 50), (35.9, 72, 50), (36.5, 77, 56), (37.5, 53, 34), (37.5, 60, 34),  # Title: the answer on each line, F on the stop
]


def compose():
    ir_big = make_ir(4.2, rng, predelay=0.03, lp=5200)
    # The brightness and level rides are the score's real shape: closed night, opening to the flip.
    cutoff = curve([(0, 760), (3, 900), (8.5, 1200), (13, 980), (22.5, 1080), (26, 1100), (28.9, 1250), (30.2, 1700), (30.9, 3200), (31, 6000),
                    (34.3, 2800), (34.5, 1300), (40, 700)])
    level = curve([(0, -14), (3, -8.5), (7, -8), (8.5, -6.5), (13, -8), (22.5, -7.5), (28.9, -7.5), (30.2, -6.5), (30.9, -4.5), (31, -1), (34.1, -2),
                   (34.5, -6), (36.5, -8), (39.6, -26), (40, -60)])

    pad = np.zeros((N, 2))
    sub = np.zeros(N)
    for start, end, midis in CHORDS:
        # Pads cross over just before the cut, so the new harmony is established on the frame.
        a = max(0, start - 0.28)
        length = (min(40, end + 0.55) - a)
        n = int(length * SR)
        sn = int(a * SR)
        fade = 1.4 if start == 0 else 0.5
        gate = ar(n, fade, 0.7 if end < 40 else 0.3)
        # The lowest note is a plain, quiet sine: weight without the limiter paying for it.
        pad[sn:sn + n] += additive_pad(midis[1:], n / SR, cutoff[sn:sn + n], rng) * gate[:, None]
        sub[sn:sn + n] += np.sin(2 * np.pi * float(hz(midis[0] + 12)) * tgrid(n / SR)) * gate
    pad = sos('highpass', 70, pad) * db(level)[:, None]
    sub = sub * 0.1 * db(level)
    pad = pad + np.stack([sub, sub], 1)

    piano_dry = fluid_render([(t, 1.8 if v < 70 else 2.4, m, v) for t, m, v in PIANO], program=0)
    # Felt the sampled grand: tame the hammer top, keep it dark until the flip opens it up.
    felt = sos('lowpass', 3400, piano_dry, 1)
    open_mix = curve([(0, 0), (30.8, 0), (31, 1), (34.3, 0.6), (34.5, 0)])[:, None]
    piano_dry = felt * (1 - open_mix) + piano_dry * open_mix

    # Air: a band of noise that climbs into the flip, breaks on it and falls back to the title.
    riser = shaped_noise(40, lambda t: float(np.interp(t, [0, 28, 31, 34.5, 40], [500, 700, 6800, 2600, 500])), 1.1, rng)
    side = shaped_noise(40, lambda t: float(np.interp(t, [0, 28, 31, 34.5, 40], [520, 720, 7200, 2500, 520])), 1.1, rng)
    air_gain = db(curve([(0, -60), (2, -44), (28, -42), (30.92, -17), (31.1, -20), (34.3, -33), (34.7, -46), (39, -62), (40, -80)]))
    air = np.stack([riser, side], 1) * air_gain[:, None]

    # Shimmer: quiet octave-up sines only while the light is on.
    shimmer = Bus()
    for i, m in enumerate([84, 89, 91, 96, 98]):
        t = tgrid(3.3)
        s = np.sin(2 * np.pi * float(hz(m)) * t + i) * (0.6 + 0.4 * np.sin(2 * np.pi * (0.9 + i * 0.27) * t + i)) * ar(len(t), 0.02 + 0.05 * i, 1.5)
        shimmer.add(s, 31.0, 0.016, pan=[-.7, .5, -.3, .8, -.1][i])

    wet = reverb(pad * 0.3 + piano_dry * 0.5 + shimmer.x + air * 0.4, ir_big)
    return {'pad': pad * 0.82, 'piano': piano_dry * 1.9, 'air_shimmer': air + shimmer.x, 'reverb': wet * 0.6}


if __name__ == '__main__':
    print(finish(compose(), sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent / 'out'))
