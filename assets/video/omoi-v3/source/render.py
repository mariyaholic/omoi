"""Omoi v3 film renderer: picture from capture/*.png, score from source/audio, viewing copy.

Usage, from anywhere:
  python source/render.py            full render: score, picture, omoi-v3.mp4, storyboard.jpg, qa/camera.json
  python source/render.py stills     storyboard.jpg only
  python source/render.py frame OUT.png SECONDS [SECONDS...]
"""
from functools import lru_cache
from pathlib import Path
import json
import math
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / 'build'
W, H, FPS, DUR = 1920, 1080, 30, 40
FRAMES = FPS * DUR
CRF = 21
P = {'night': '#120c24', 'blue': '#22204a', 'mid': '#33407a', 'light': '#8fa0dd', 'ink': '#cfd6f4', 'paper': '#f3efe4', 'pink': '#f08868', 'gold': '#f4cf8a'}
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
AREA = (57, 243, 1863, 975)
AW, AH = AREA[2] - AREA[0], AREA[3] - AREA[1]
ASPECT = AH / AW
PROVENANCE = json.loads((ROOT / 'source' / 'capture-provenance.json').read_text(encoding='utf-8'))
DSF = {s['file'][:-4]: s['device_scale_factor'] for s in PROVENANCE['shots']}
CLIP = {s['file'][:-4]: s['clip_css'] or [0, 0] for s in PROVENANCE['shots']}

# (start, end, caption, what the picture does). The score uses the same hit points; see main().
SCENES = [
    (0, 3, 'Find it again', 'Original pixel night train and city'),
    (3, 7, 'Browse', 'Real History, Mariyaholic-98; camera travels down all eight rows'),
    (7, 8.5, 'Search', 'Real search box: typed t-r-a-i-n, camera pulls back to the list'),
    (8.5, 11, 'Find', 'Enter pressed: the real list filters to one item, camera moves in'),
    (11, 13, 'Find', 'Close-up: the result fills the frame'),
    (13, 15.5, 'Inspect', 'Real values: unknown duration, no recorded playback, one opening'),
    (15.5, 18, 'Check openings', 'One camera move down the same sheet to the tab-time caveat'),
    (18, 21, 'Tag', 'Real editor, tags typed line by line, labeled unsaved'),
    (21, 22.5, 'Choose sources', 'Real Sources page: two demo profiles ticked'),
    (22.5, 26, 'Choose sources', 'Real collection confirmation, whole rows framed'),
    (26, 28.9, 'Switch themes', 'Real five-theme controls, dark'),
    (28.9, 31, 'Switch themes', 'Confluence selected: the light page dissolves in inside the dark stage'),
    (31, 34.5, 'Keep browsing', 'The stage dissolves to paper; same catalogue in Confluence'),
    (34.5, 40, 'omoi', 'Pixel train, what omoi is, status, disclosure'),
]
CUTS = [s[0] for s in SCENES] + [DUR]
# Dissolve spans (seconds); each is centred on its hit point.
THEME_FADE = (28.6, 29.2)
STAGE_FADE = (30.7, 31.3)
END_FADE = (34.2, 34.8)
camera_log = {}


def smooth(p):
    p = max(0, min(1, p))
    return p * p * (3 - 2 * p)


def ramp(t, span):
    return smooth((t - span[0]) / (span[1] - span[0]))


def pixel_text(im, xy, text, size=16, col='ink', anchor='la'):
    """One-bit text on the 640x360 art grid, set by ink width so spacing stays even at this size."""
    font = ImageFont.truetype(FONT, size)
    gap, word = (1 if size < 13 else max(2, round(size / 11))), round(size * .42)
    top = font.getbbox('Hg')[1]
    height = size + 8
    glyphs = []
    for ch in text:
        cell = Image.new('1', (size * 2, height))
        ImageDraw.Draw(cell).text((size // 2, -top), ch, font=font, fill=1)
        box = cell.getbbox()
        glyphs.append(cell.crop((box[0], 0, box[2], height)) if box else None)
    width = sum(g.width + gap if g else word for g in glyphs) - gap
    mask = Image.new('1', (width, height))
    pen = 0
    for g in glyphs:
        if g:
            mask.paste(g, (pen, 0))
        pen += g.width + gap if g else word
    mask = mask.crop(mask.getbbox())
    x, y = xy
    if anchor == 'center':
        x -= mask.width // 2
    elif anchor == 'right':
        x -= mask.width
    assert 0 <= x and 0 <= y and x + mask.width <= im.width and y + mask.height <= im.height, (text, xy, mask.size)
    im.paste(P.get(col, col), (int(x), int(y)), mask)


STARS_OPEN = [(46, 152), (320, 34), (370, 80), (414, 128), (464, 112), (530, 40), (562, 61), (601, 155)]
STARS_END = [(46, 90), (100, 150), (60, 40), (530, 30), (562, 61), (590, 110), (601, 155), (30, 128)]


@lru_cache(maxsize=1)
def world_back():
    im = Image.new('RGB', (640, 360), P['night'])
    d = ImageDraw.Draw(im)
    for y in range(130, 275):
        for x in range((y % 2) * 2, 640, 4):
            if (y - 130) / 145 > ((x * 17 + y * 13) % 16) / 16:
                d.point((x, y), fill=P['blue'])
    # Quiet city silhouettes, not stock footage.
    for i in range(18):
        x = i * 39 - 12
        hh = 30 + (i * 29) % 62
        d.rectangle((x, 252 - hh, x + 26, 256), fill=P['blue'])
        for wy in range(260 - hh, 245, 10):
            for wx in range(x + 5, x + 24, 8):
                if (wx + wy + i) % 3 == 0:
                    d.rectangle((wx, wy, wx + 2, wy + 3), fill=P['mid'])
    d.rectangle((0, 257, 639, 359), fill=P['night'])
    d.line((0, 280, 639, 280), fill=P['light'])
    d.line((0, 290, 639, 290), fill=P['mid'])
    for x in range(-30, 650, 22):
        d.line((x, 291, x + 12, 291), fill=P['mid'])
    return im


def world(t, ending=False):
    im = world_back().copy()
    d = ImageDraw.Draw(im)
    # Stars stay clear of the copy: next to a word, one reads as an authored period.
    for x, y in (STARS_END if ending else STARS_OPEN):
        d.point((x, y), fill=P['ink'] if int(t * 1.2 + x) % 3 else P['light'])
        if x in (46, 562):
            d.line((x - 1, y, x + 1, y), fill=P['light'])
    # A miniature train gives the synthetic search title a visual memory.
    xx = int(100 + 105 * smooth(t / 3)) if not ending else int(205 + 55 * smooth(t / 2))
    for car in range(3):
        x = xx + car * 76
        d.rectangle((x, 236, x + 70, 273), fill=P['mid'])
        d.rectangle((x + 3, 239, x + 67, 249), fill=P['light'])
        d.line((x, 261, x + 70, 261), fill=P['pink'])
        for j in range(4):
            d.rectangle((x + 7 + j * 15, 242, x + 16 + j * 15, 252), fill=P['gold'])
        for dx in (13, 56):
            d.rectangle((x + dx, 274, x + dx + 7, 279), fill=P['night'])
            d.line((x + dx, 279, x + dx + 7, 279), fill=P['light'])
        if car < 2:
            d.line((x + 70, 266, x + 76, 266), fill=P['light'])
    if ending:
        pixel_text(im, (320, 26), 'omoi', 57, 'paper', 'center')
        if t >= .8:
            pixel_text(im, (320, 96), 'Local watch history for the PC', 20, 'ink', 'center')
        if t >= 1.4:
            pixel_text(im, (320, 128), 'Windows · development preview', 15, 'light', 'center')
        pixel_text(im, (320, 324), 'APP CAPTURE / SYNTHETIC DATA', 11, 'light', 'center')
    else:
        pixel_text(im, (38, 32), 'OMOI', 12, 'pink')
        pixel_text(im, (38, 96), 'Find it again', 29, 'paper')
    return im.resize((W, H), Image.Resampling.NEAREST)


@lru_cache(maxsize=16)
def stage(title, unsaved=False, light=False):
    im = Image.new('RGB', (640, 360), P['paper'] if light else P['night'])
    d = ImageDraw.Draw(im)
    ink = 'night' if light else 'ink'
    edge = 'mid' if light else 'light'
    pixel_text(im, (22, 18), 'OMOI / UNSAVED PREVIEW' if unsaved else 'OMOI', 10, 'mid' if light else 'pink')
    pixel_text(im, (22, 39), title, 22, ink)
    d.rectangle((15, 76, 624, 329), outline=P[edge])
    d.rectangle((17, 78, 622, 327), outline=P['mid'])
    pixel_text(im, (22, 341), 'APP CAPTURE', 9, 'mid' if light else 'light')
    pixel_text(im, (618, 341), 'SYNTHETIC DATA', 9, 'mid' if light else 'light', 'right')
    return im.resize((W, H), Image.Resampling.NEAREST)


@lru_cache(maxsize=None)
def source(name):
    return Image.open(ROOT / 'capture' / f'{name}.png').convert('RGB')


@lru_cache(maxsize=8)
def resample(name, x0, y0, x1, y1):
    return source(name).resize((AW, AH), Image.Resampling.LANCZOS, box=(x0, y0, x1, y1))


def view(name, cx, cy, w):
    """The film's camera: a float box in CSS px over one screenshot, resampled to the stage window.

    The box always has the window's aspect, so a shot fills the window; no side columns."""
    h = w * ASPECT
    dsf, ox, oy = DSF[name], CLIP[name][0], CLIP[name][1]
    box = [(cx - w / 2 - ox) * dsf, (cy - h / 2 - oy) * dsf, (cx + w / 2 - ox) * dsf, (cy + h / 2 - oy) * dsf]
    img = source(name)
    assert box[0] >= -1e-6 and box[1] >= -1e-6 and box[2] <= img.width + 1e-6 and box[3] <= img.height + 1e-6, (name, box, img.size)
    box = [max(0.0, box[0]), max(0.0, box[1]), min(float(img.width), box[2]), min(float(img.height), box[3])]
    scale = AW / (box[2] - box[0])
    low, high = camera_log.get(name, (scale, scale))
    camera_log[name] = (min(low, scale), max(high, scale))
    return resample(name, *[round(v, 3) for v in box])


def glide(t, keys):
    """Eased camera path. keys: (time, cx, cy, w); width eases on a log scale so zoom feels even."""
    if t <= keys[0][0]:
        return keys[0][1:]
    for a, b in zip(keys, keys[1:]):
        if t < b[0]:
            p = smooth((t - a[0]) / (b[0] - a[0]))
            return (a[1] + (b[1] - a[1]) * p, a[2] + (b[2] - a[2]) * p, math.exp(math.log(a[3]) + (math.log(b[3]) - math.log(a[3])) * p))
    return keys[-1][1:]


def app(title, window, unsaved=False, light=False):
    im = stage(title, unsaved, light).copy()
    im.paste(window, AREA[:2])
    return im


def box_keys(t, x0, y0, x1):
    return (t, (x0 + x1) / 2, y0 + (x1 - x0) * ASPECT / 2, x1 - x0)


# Framing, in CSS px. Each box's edges fall on empty space or a page edge.
SEARCH_TIGHT = (257, 100, 1120)     # search box through the first row, ends above the next row's text
SEARCH_WIDE = (0, 82, 1105)         # whole window below the page title and logo
SHEET_RIGHT = 1150                  # details and editor sheets run to the window edge
SHEET_W = 417                       # inside the sheet's own left border
SOURCES = (257.5, 150, 1111)        # both whole profile rows through the dialog; stops above the next box


def history_browse(t):
    # Starts on the whole window, ends on the main column, whose text is larger.
    return view('03-history', *glide(t, [box_keys(3.5, 0, 0, 1120), (6.85, 684, 1365 - 932 * ASPECT / 2, 932)]))


def search_shot(t):
    """Typed frames land on the score's five notes; Enter lands on 8.5 s."""
    if t < 7.13:
        name = '04-search-0-empty'
    elif t < 7.27:
        name = '04-search-1-t'
    elif t < 7.43:
        name = '04-search-2-tr'
    elif t < 7.57:
        name = '04-search-3-tra'
    elif t < 7.73:
        name = '04-search-4-trai'
    elif t < 8.5:
        name = '04-search-5-train'
    else:
        name = '05-search-result'
    keys = [box_keys(7.75, *SEARCH_TIGHT), box_keys(8.4, *SEARCH_WIDE), box_keys(9.1, *SEARCH_WIDE), box_keys(10.2, 257, 100, 1120)]
    return view(name, *glide(t, keys))


def closeup(t):
    return view('05-search-result-close', *glide(t, [(11, 478, 366, 440), (13, 478, 366, 426)]))


def details(t):
    sheet = lambda w: SHEET_RIGHT - w / 2
    return view('06-details', *glide(t, [(13, sheet(SHEET_W), 447, SHEET_W), (14.6, sheet(SHEET_W), 447, SHEET_W), (15.05, sheet(640), 655, 640),
                                         (15.5, sheet(SHEET_W), 864, SHEET_W), (18, sheet(SHEET_W), 864, SHEET_W)]))


def tags(t):
    name = '08-edit-tags-0-empty' if t < 18.6 else '08-edit-tags-1-miniatures' if t < 19.4 else '08-edit-tags-2-night-train'
    return view(name, SHEET_RIGHT - SHEET_W / 2, 566, SHEET_W)


def sources(t):
    if t < 22.5:
        name = '01-sources-0' if t < 21.35 else '01-sources-1' if t < 21.8 else '01-sources-2'
    else:
        name = '02-confirmation'
    return view(name, *box_keys(0, *SOURCES)[1:])


def settings_dark():
    return view('09-appearance', 218 + 466, 9 + 932 * ASPECT / 2, 932)


def settings_light():
    # Offset so the five theme cards hold still through the dissolve.
    return view('09-appearance-confluence', 218 + 466, 2.8 + 932 * ASPECT / 2, 932)


def confluence(t):
    return view('10-confluence', *glide(t, [(31.3, 575, 1150 * ASPECT / 2, 1150), (34.4, 575, 1150 * ASPECT / 2 + 300, 1150)]))


def blend(a, b, p):
    return a if p <= 0 else b if p >= 1 else Image.blend(a, b, p)


def frame(t):
    if t < 3:
        return world(t)
    if t < 7:
        return app('Browse', history_browse(t))
    if t < 8.5:
        return app('Search', search_shot(t))
    if t < 11:
        return app('Find', search_shot(t))
    if t < 13:
        return app('Find', closeup(t))
    if t < 15.5:
        return app('Inspect', details(t))
    if t < 18:
        return app('Check openings', details(t))
    if t < 21:
        return app('Tag', tags(t), unsaved=True)
    if t < 26:
        return app('Choose sources', sources(t))
    if t < STAGE_FADE[0]:
        # Confluence first appears inside the dark stage, so the light never jumps at full frame.
        return app('Switch themes', blend(settings_dark(), settings_light(), ramp(t, THEME_FADE)))
    if t < STAGE_FADE[1]:
        return blend(app('Switch themes', settings_light()), app('Keep browsing', confluence(t), light=True), ramp(t, STAGE_FADE))
    if t < END_FADE[0]:
        return app('Keep browsing', confluence(t), light=True)
    if t < END_FADE[1]:
        return blend(app('Keep browsing', confluence(t), light=True), world(t - 34.5, True), ramp(t, END_FADE))
    return world(t - 34.5, True)


def stills():
    """One frame per scene, chosen where the scene has settled."""
    stamps = [1.8, 5.0, 7.74, 10.5, 12.0, 14.0, 17.0, 20.0, 22.0, 24.0, 27.5, 30.0, 33.0, 38.0]
    assert len(stamps) == len(SCENES)
    cols, cw, ch = 3, 640, 410
    sheet = Image.new('RGB', (cols * cw, ((len(stamps) + cols - 1) // cols) * ch), P['night'])
    d = ImageDraw.Draw(sheet)
    font = ImageFont.truetype(FONT, 22)
    for i, t in enumerate(stamps):
        x, y = (i % cols) * cw, (i // cols) * ch
        sheet.paste(frame(t).resize((cw, 360), Image.Resampling.LANCZOS), (x, y))
        a, b, caption, _ = SCENES[i]
        d.text((x + 14, y + 370), f'{i + 1:02d}  {t:04.1f}s  [{a:g}-{b:g}]  {caption}', font=font, fill=P['ink'])
    sheet.save(ROOT / 'storyboard.jpg', quality=92)


def score():
    sys.path.insert(0, str(ROOT / 'source' / 'audio'))
    import build
    assert build.CUTS == CUTS, 'score and picture disagree on the cut list'
    print(build.finish(build.compose(), BUILD / 'score'), flush=True)


def write_camera_log():
    rows = {}
    for name, (low, high) in sorted(camera_log.items()):
        rows[name] = {'device_scale_factor': DSF[name], 'scale_min': round(low, 4), 'scale_max': round(high, 4)}
    (ROOT / 'qa').mkdir(exist_ok=True)
    (ROOT / 'qa' / 'camera.json').write_text(json.dumps({'window_px': [AW, AH], 'note': 'Output px per capture px, over every rendered frame. Above 1.0 is an upscale.', 'shots': rows}, indent=2) + '\n', encoding='utf-8')


def render():
    BUILD.mkdir(exist_ok=True)
    score()
    stills()
    silent = BUILD / 'silent.mp4'
    args = ['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(FPS), '-i', '-', '-an',
            '-vf', 'scale=in_range=full:out_range=tv:out_color_matrix=bt709:flags=lanczos+accurate_rnd+full_chroma_int,format=yuv420p',
            '-c:v', 'libx264', '-preset', 'slow', '-crf', str(CRF), '-tune', 'animation', '-threads', '4',
            '-x264-params', 'colorprim=bt709:transfer=bt709:colormatrix=bt709:range=tv', '-bitexact', str(silent)]
    proc = subprocess.Popen(args, stdin=subprocess.PIPE)
    camera_log.clear()
    for i in range(FRAMES):
        proc.stdin.write(frame(i / FPS).tobytes())
        if i % 150 == 0:
            print(f'{i}/{FRAMES}', flush=True)
    proc.stdin.close()
    assert proc.wait() == 0
    write_camera_log()
    subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(silent), '-i', str(BUILD / 'score' / 'score.m4a'),
                    '-map', '0:v:0', '-map', '1:a:0', '-c', 'copy', '-movflags', '+faststart', '-bitexact', str(ROOT / 'omoi-v3.mp4')], check=True)
    print('RENDER COMPLETE', flush=True)


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == 'stills':
        stills()
    elif len(sys.argv) > 1 and sys.argv[1] == 'frame':
        stem = Path(sys.argv[2])
        for s in sys.argv[3:]:
            frame(float(s)).save(stem.with_name(f'{stem.stem}-{float(s):06.3f}{stem.suffix}'))
    else:
        render()
