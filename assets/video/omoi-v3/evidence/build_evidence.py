"""Build evidence/ from the two delivered mp4 files, never from renderer frames.

Writes sharpness.json, transitions.json, side-by-side crops and frame-grab sheets.
Usage, from work/: python evidence/build_evidence.py
"""
from pathlib import Path
import json
import re
import subprocess

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage

HERE = Path(__file__).resolve().parent
WORK = HERE.parent
V2 = WORK.parent.parent / 'pr7' / 'assets' / 'video' / 'omoi-v2' / 'omoi-v2.mp4'
V3 = WORK / 'omoi-v3' / 'omoi-v3.mp4'
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
W, H, FPS = 1920, 1080, 30
AREA = (57, 243, 1863, 975)

# name, v2 second, v3 second, a text region both frames fill, what is compared.
# Crops show the whole stage window from each film at 1:1; metrics run on the window and on the text region.
PAIRS = [
    ('search-box-typed', 8.0, 7.74, (300, 300, 1500, 700), 'Search box with "train" and the filter row'),
    ('search-result-title', 12.0, 12.0, (420, 480, 1620, 880), 'Close-up of the result title; same timestamp'),
    ('details-values', 15.0, 14.0, (300, 300, 1500, 900), 'Duration, playback and openings values'),
    ('openings-caveat', 19.0, 17.0, (300, 300, 1500, 900), 'Recent openings and the tab-time caveat'),
    ('tag-editor', 23.0, 20.0, (300, 300, 1500, 900), 'Unsaved tag editor'),
    ('sources-dialog', 27.0, 24.0, (300, 300, 1500, 900), 'Collection confirmation dialog'),
    ('themes', 30.5, 27.5, (300, 300, 1500, 900), 'Five theme controls'),
]
# Control: the city and train are the same art at the same second in both films.
# If the metric were only reacting to the encoder, this ratio would move too.
CONTROL = ('control-pixel-art-city', 1.8, 1.8, (0, 480, 1920, 880), 'Control: pixel city and train, same art at the same second')
# Sheet ranges: the requested v2 ranges, extended where the v3 scene runs on.
RANGES = [(7, 9), (11, 13), (13, 21), (21, 29), (29, 35)]
SHEET_STEP = 5  # Every fifth film frame: 6 frames per second of film.


def decode(path, wanted):
    """Decode a whole file in order; returns {frame index: RGB array} for the wanted frames."""
    size = W * H * 3
    proc = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:v:0', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'], stdout=subprocess.PIPE)
    out, i = {}, 0
    while True:
        raw = proc.stdout.read(size)
        if len(raw) < size:
            break
        if i in wanted:
            out[i] = np.frombuffer(raw, np.uint8).reshape((H, W, 3)).copy()
        i += 1
    assert proc.wait() == 0 and i == 1200, (path, i)
    return out


def mean_luma(path):
    """Mean of each decoded frame's luma, from ffmpeg's gray output."""
    size = W * H
    proc = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:v:0', '-f', 'rawvideo', '-pix_fmt', 'gray', '-'], stdout=subprocess.PIPE)
    means = []
    while True:
        raw = proc.stdout.read(size)
        if len(raw) < size:
            break
        means.append(float(np.frombuffer(raw, np.uint8).mean()))
    assert proc.wait() == 0 and len(means) == 1200, (path, len(means))
    return means


def luma_of(rgb):
    return 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]


def sharpness(gray):
    g = gray.astype(float)
    sx, sy = ndimage.sobel(g, 1) / 8, ndimage.sobel(g, 0) / 8
    mag = np.hypot(sx, sy)
    return {'variance_of_laplacian': round(float(ndimage.laplace(g).var()), 2), 'mean_gradient_magnitude': round(float(mag.mean()), 3),
            'p99_gradient_magnitude': round(float(np.percentile(mag, 99)), 3)}


def crop(a, box):
    return a[box[1]:box[3], box[0]:box[2]]


def label(im, text, xy, size=22):
    ImageDraw.Draw(im).text(xy, text, font=ImageFont.truetype(FONT, size), fill='#f4cf8a', stroke_width=2, stroke_fill='#000000')


def side_by_side(name, a, b, box, ta, tb):
    """The whole stage window at 1:1, v2 above v3, so the same size and place on screen line up."""
    gap = 12
    w, h = box[2] - box[0], box[3] - box[1]
    im = Image.new('RGB', (w, h * 2 + gap), '#000000')
    im.paste(Image.fromarray(crop(a, box)), (0, 0))
    im.paste(Image.fromarray(crop(b, box)), (0, h + gap))
    label(im, f'v2 at {ta:g} s', (10, 8))
    label(im, f'v3 at {tb:g} s', (10, h + gap + 8))
    im.save(HERE / f'compare-{name}.png')


def transition_stats(means):
    deltas = np.abs(np.diff(means))
    top = int(np.argmax(deltas))
    return {'max_frame_to_frame_luma_change_0_255': round(float(deltas[top]), 2), 'at_seconds': round((top + 1) / FPS, 3),
            'luma_before': round(means[top], 1), 'luma_after': round(means[top + 1], 1),
            'frames_over_20_levels': int((deltas > 20).sum()), 'frames_over_60_levels': int((deltas > 60).sum())}


def sheets(frames):
    font = ImageFont.truetype(FONT, 18)
    made = []
    for a, b in RANGES:
        idx = list(range(int(a * FPS), int(b * FPS) + 1, SHEET_STEP))
        cols, cw, ch = 6, 480, 270
        sheet = Image.new('RGB', (cols * cw, ((len(idx) + cols - 1) // cols) * (ch + 26)), '#000000')
        d = ImageDraw.Draw(sheet)
        for n, i in enumerate(idx):
            x, y = (n % cols) * cw, (n // cols) * (ch + 26)
            sheet.paste(Image.fromarray(frames[i]).resize((cw, ch), Image.Resampling.LANCZOS), (x, y))
            d.text((x + 6, y + ch + 3), f'{i / FPS:06.3f} s  frame {i}', font=font, fill='#f4cf8a')
        path = HERE / f'frames-{a:02d}-{b:02d}s.jpg'
        sheet.save(path, quality=88)
        made.append({'file': path.name, 'range_s': [a, b], 'frames': len(idx), 'frames_per_film_second': FPS / SHEET_STEP})
    return made


def main():
    pair_frames_v2 = {round(p[1] * FPS) for p in PAIRS + [CONTROL]}
    pair_frames_v3 = {round(p[2] * FPS) for p in PAIRS + [CONTROL]}
    sheet_frames = {i for a, b in RANGES for i in range(int(a * FPS), int(b * FPS) + 1, SHEET_STEP)}
    v2, v2_means = decode(V2, pair_frames_v2), mean_luma(V2)
    v3, v3_means = decode(V3, pair_frames_v3 | sheet_frames), mean_luma(V3)
    rows = []
    for name, ta, tb, box, what in PAIRS + [CONTROL]:
        a, b = v2[round(ta * FPS)], v3[round(tb * FPS)]
        if not name.startswith('control'):
            side_by_side(name, a, b, AREA, ta, tb)
        ma, mb = sharpness(luma_of(crop(a, box))), sharpness(luma_of(crop(b, box)))
        row = {'shot': name, 'what': what, 'v2_second': ta, 'v3_second': tb, 'region_xyxy': list(box), 'v2': ma, 'v3': mb,
               'ratio_v3_over_v2': {k: round(mb[k] / ma[k], 2) for k in ma}}
        if not name.startswith('control'):
            wa, wb = sharpness(luma_of(crop(a, AREA))), sharpness(luma_of(crop(b, AREA)))
            row['stage_window'] = {'region_xyxy': list(AREA), 'v2': wa, 'v3': wb, 'ratio_v3_over_v2': {k: round(wb[k] / wa[k], 2) for k in wa}}
        rows.append(row)
    (HERE / 'sharpness.json').write_text(json.dumps({
        'method': 'Decoded frames of omoi-v2.mp4 and omoi-v3.mp4; Rec.709 luma of the same on-screen region at 1:1; scipy Laplacian and Sobel. '
                  'Framing differs between films, so read these with the side-by-side crops; the p99 gradient is the least content-dependent figure.',
        'shots': rows}, indent=2) + '\n', encoding='utf-8')
    (HERE / 'transitions.json').write_text(json.dumps({
        'method': 'Mean of each decoded frame, ffmpeg gray output (0-255).',
        'v2_film': transition_stats(v2_means), 'v3_film': transition_stats(v3_means),
        'v2_jump_into_light_at_32_s': jump(v2_means, 32), 'v2_jump_out_of_light_at_35_s': jump(v2_means, 35),
        'v3_confluence_inside_dark_stage_28_6_to_29_2_s': transition_stats_window(v3_means, 28.5, 29.3),
        'v3_stage_opens_to_paper_30_7_to_31_3_s': transition_stats_window(v3_means, 30.6, 31.4),
        'v3_light_to_night_34_2_to_34_8_s': transition_stats_window(v3_means, 34.1, 34.9),
        'v3_luma_every_film_frame_28_to_36_s': [round(v, 1) for v in v3_means[28 * FPS:36 * FPS]]}, indent=2) + '\n', encoding='utf-8')
    (HERE / 'v2-audio.json').write_text(json.dumps(v2_audio(), indent=2) + '\n', encoding='utf-8')
    (HERE / 'v2-scale-factors.json').write_text(json.dumps(v2_scales(), indent=2) + '\n', encoding='utf-8')
    a, b = crop(luma_of(v2[round(12 * FPS)]), AREA), crop(luma_of(v3[round(12 * FPS)]), AREA)
    (HERE / 'closeup-fill.json').write_text(json.dumps({'method': 'Stage window at 12.0 s: pixels more than 10 luma levels from the most common level, and the height of the rows holding any such pixel in a 1 percent or denser run.',
                                                        'v2': fill(a), 'v3': fill(b)}, indent=2) + '\n', encoding='utf-8')
    made = sheets(v3)
    (HERE / 'frame-sheets.json').write_text(json.dumps(made, indent=2) + '\n', encoding='utf-8')
    for r in rows:
        print(r['shot'], r['ratio_v3_over_v2'])
    print(json.dumps(json.loads((HERE / 'transitions.json').read_text())['v2_film']), json.dumps(json.loads((HERE / 'transitions.json').read_text())['v3_film']))


def transition_stats_window(means, a, b):
    i = round(a * FPS)
    stats = transition_stats(means[i:round(b * FPS)])
    stats['at_seconds'] = round(stats['at_seconds'] + a, 3)
    return stats | {'window_s': [a, b]}


def jump(means, t):
    i = round(t * FPS)
    return {'change_0_255': round(abs(means[i] - means[i - 1]), 2), 'luma_before': round(means[i - 1], 1), 'luma_after': round(means[i], 1)}


def fill(window):
    window = window.round().astype(int)
    mode = np.bincount(window.ravel()).argmax()
    ink = np.abs(window - mode) > 10
    rows = np.where(ink.mean(1) > 0.01)[0]
    return {'ink_pixels_percent': round(float(ink.mean() * 100), 2), 'occupied_height_percent': round(float((rows.max() - rows.min() + 1) / window.shape[0] * 100), 1)}


def v2_scales():
    """Largest upscale of each v2 shot, from the crop boxes and zooms in its render.py. Derived, not measured on video."""
    aw, ah = 1806, 732
    shots = {'03-history': ((1145, 510), 1.025), '04-search-input': ((885, 335), 1), '05-search-result wide': ((897, 416), 1.02), '05-search-result close': ((692, 135), 1.02),
             '06-details': ((380, 145), 1.02), '07-opening-evidence': ((385, 175), 1), '08-edit-tags': ((380, 175), 1), '02-confirmation': ((670, 380), 1.04),
             '09-appearance': ((900, 385), 1.025), '10-confluence': ((1145, 505), 1.025)}
    return {k: round(min(aw / w, ah / h) * z, 3) for k, ((w, h), z) in shots.items()}


def v2_audio():
    run = lambda *a: subprocess.run(a, capture_output=True, text=True, check=True)
    probe = json.loads(run('ffprobe', '-v', 'error', '-select_streams', 'a:0', '-show_entries', 'stream=codec_name,sample_rate,duration,bit_rate', '-of', 'json', str(V2)).stdout)
    tail = run('ffmpeg', '-hide_banner', '-nostats', '-i', str(V2), '-map', '0:a:0', '-af', 'ebur128=peak=true', '-f', 'null', '-').stderr
    tail = tail[tail.rindex('Summary:'):]
    return {'file': V2.name, **probe['streams'][0], 'integrated_lufs': float(re.search(r'I:\s+(-?[\d.]+) LUFS', tail)[1]),
            'lra_lu': float(re.search(r'LRA:\s+(-?[\d.]+) LU', tail)[1]), 'true_peak_dbtp': float(re.search(r'Peak:\s+(-?[\d.]+) dBFS', tail)[1])}


if __name__ == '__main__':
    main()
