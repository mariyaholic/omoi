"""Capture the real omoi UI for the v3 film, on synthetic data only.

Rebuilds a sandbox under ../../sandbox: two fake Chromium history files, one `omoi collect`,
then drives the connected UI with Playwright (the approach of omoi's scripts/verify_ui.py,
adapted to Linux Chromium) and writes capture/*.png and source/capture-provenance.json.
Every omoi process gets HOME, XDG_DATA_HOME, XDG_CONFIG_HOME and OMOI_HOME inside the sandbox.
The metadata save is never performed; the run fails if the catalogue changes.

Usage: python source/capture.py [--binary PATH]
"""
import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path
import queue
import shutil
import sqlite3
import subprocess
import threading
import calendar

from PIL import Image
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT.parent
SANDBOX = WORK / 'sandbox'
OUT = ROOT / 'capture'
COMMIT = 'cc530f0af90128c7ff933f80d782d7cfc2a31cf4'

# v2's client area: its 1165 px screenshots include a 15 px scrollbar, hidden here.
WIDTH, HEIGHT = 1150, 747
# omoi centres a page shorter than its window. At this height the one-item result stays put,
# so the search box does not jump when the list filters.
SEARCH_HEIGHT = 530
SOURCES_HEIGHT = 700

# (title, browser, UTC opening, tab seconds). The first four are read from the v2 captures;
# rows five to eight never appear in them, so they are newly authored synthetic titles.
CATALOGUE = [
    ('Night sky timelapse above the ridge', 'edge', (2026, 11, 4, 18, 0), 212),
    ('Building a miniature night train', 'chrome', (2026, 11, 4, 18, 0), 65),
    ('Making a loop from field recordings', 'edge', (2026, 11, 3, 17, 0), 148),
    ('How Tokyo sounds after midnight', 'chrome', (2026, 11, 3, 17, 0), 97),
    ('Drawing rain on a small window', 'edge', (2026, 11, 2, 21, 0), 183),
    ('Repairing an old radio by lamplight', 'chrome', (2026, 11, 2, 21, 0), 126),
    ('Mapping the last bus of the night', 'edge', (2026, 11, 1, 20, 0), 74),
    ('A quiet walk past the harbour cranes', 'chrome', (2026, 11, 1, 20, 0), 240),
]
WEBKIT_EPOCH = 11644473600


def fresh_sandbox():
    """Recreate the sandbox and return the environment every omoi process runs under."""
    assert SANDBOX.parent == WORK and SANDBOX.name == 'sandbox'
    if SANDBOX.exists():
        shutil.rmtree(SANDBOX)
    env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'TZ': 'UTC'}
    for key, name in (('HOME', 'home'), ('XDG_DATA_HOME', 'xdg-data'), ('XDG_CONFIG_HOME', 'xdg-config'), ('OMOI_HOME', 'omoi-home')):
        path = SANDBOX / name
        path.mkdir(parents=True)
        env[key] = str(path)
    return env


def write_histories(env):
    """Two fake Chromium profiles, found through omoi's normal discovery under XDG_CONFIG_HOME."""
    config = Path(env['XDG_CONFIG_HOME'])
    for browser, root, profile in (('chrome', 'google-chrome', 'Demo Chrome'), ('edge', 'microsoft-edge', 'Demo Edge')):
        folder = config / root / profile
        folder.mkdir(parents=True)
        with closing(sqlite3.connect(folder / 'History')) as c:
            c.executescript('CREATE TABLE urls(id INTEGER PRIMARY KEY,url TEXT,title TEXT);'
                            'CREATE TABLE visits(id INTEGER PRIMARY KEY,url INTEGER,visit_time INTEGER,visit_duration INTEGER);')
            for i, (title, owner, when, tab) in enumerate(CATALOGUE, 1):
                if owner == browser:
                    # Synthetic video ids: "omoidemo" plus three digits, the shape of a YouTube id only.
                    c.execute('INSERT INTO urls VALUES(?,?,?)', (i, f'https://www.youtube.com/watch?v=omoidemo{i:03}', title))
                    stamp = calendar.timegm((*when, 0))
                    c.execute('INSERT INTO visits VALUES(?,?,?,?)', (i, i, (stamp + WEBKIT_EPOCH) * 1000000, tab * 1000000))
            c.commit()
    (Path(env['OMOI_HOME']) / 'config.toml').write_text('browsers = ["chrome", "edge"]\nlocal_utc_offset = "+00:00"\n', encoding='utf-8')


def run_omoi(binary, env, *args):
    return subprocess.run([str(binary), *args], env=env, cwd=SANDBOX, capture_output=True, text=True, check=True, timeout=120).stdout


def check_paths(binary, env):
    """Refuse to go further unless omoi itself reports only sandbox locations."""
    paths = json.loads(run_omoi(binary, env, 'paths'))
    flat = [paths[k] for k in ('home', 'config', 'db', 'logs', 'tmp', 'thumbs')] + [p for _, p in paths['browser_roots']]
    for path in flat:
        assert Path(path).resolve().is_relative_to(SANDBOX.resolve()), path
    return len(flat)


def launch_ui(binary, env):
    proc = subprocess.Popen([str(binary), 'ui', '--port', '0', '--idle-seconds', '900'], env=env, cwd=SANDBOX,
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    lines = queue.Queue()
    threading.Thread(target=lambda: lines.put(proc.stdout.readline()), daemon=True).start()
    url = lines.get(timeout=30).strip()
    assert url.startswith('http://127.0.0.1:'), 'UI did not emit a launch URL'
    return proc, url


def catalogue_state(env):
    with closing(sqlite3.connect(Path(env['OMOI_HOME']) / 'history.sqlite')) as c:
        return {'items': c.execute('SELECT count(*) FROM items').fetchone()[0],
                'browser_openings': c.execute('SELECT count(*) FROM views').fetchone()[0],
                'tags': c.execute('SELECT count(*) FROM item_tags').fetchone()[0],
                'kinds': sorted({r[0] for r in c.execute('SELECT kind FROM items')})}


class Shooter:
    def __init__(self, browser, base_url):
        self.browser, self.url = browser, base_url
        self.shots, self.layout, self.used_fonts = [], {}, {}
        self.external, self.errors = [], []
        self.storage = None
        self.page = self.context = None
        self.dsf = None

    def open(self, dsf, width=WIDTH, height=HEIGHT):
        """A new context per pixel density. Theme lives in localStorage, so it is chosen again each time."""
        self.close()
        self.dsf = dsf
        self.context = self.browser.new_context(viewport={'width': width, 'height': height}, device_scale_factor=dsf, locale='en-US',
                                                timezone_id='UTC', color_scheme='dark', reduced_motion='reduce', service_workers='block')
        self.page = self.context.new_page()
        self.page.set_default_timeout(15000)
        self.page.on('pageerror', lambda e: self.errors.append(str(e)))
        self.page.on('request', lambda r: self.external.append(r.url) if not r.url.startswith((self.origin(), 'data:', 'about:')) else None)
        self.page.goto(self.url)
        expect(self.page.locator('#content')).to_have_attribute('aria-busy', 'false')
        self.theme('Mariyaholic-98')

    def origin(self):
        return self.url.split('/', 3)[0] + '//' + self.url.split('/', 3)[2]

    def close(self):
        if self.context:
            self.context.close()
            self.context = None

    def theme(self, name):
        p = self.page
        p.locator('nav [data-page="settings"]').click()
        p.get_by_role('button', name=name, exact=True).click()
        p.locator('nav [data-page="history"]').click()
        expect(p.locator('.history-row').first).to_be_visible()

    def fit_height(self, selector=None):
        """Grow the window until nothing scrolls, so the camera can travel one real, continuous frame."""
        p = self.page
        for _ in range(4):
            need = p.evaluate('(s)=>Math.max(document.documentElement.scrollHeight, s?document.querySelector(s).scrollHeight:0)', selector)
            view = p.viewport_size
            if need <= view['height']:
                break
            p.set_viewport_size({'width': view['width'], 'height': need})
            p.wait_for_timeout(100)

    def rects(self, name, **selectors):
        out = {}
        for key, sel in selectors.items():
            box = self.page.locator(sel).first.bounding_box()
            out[key] = [round(box[k], 2) for k in ('x', 'y', 'width', 'height')]
        self.layout[name] = out

    def fonts(self, theme, **selectors):
        """The fonts Chromium actually used here; they depend on this machine, not only on omoi's CSS."""
        cdp = self.context.new_cdp_session(self.page)
        cdp.send('DOM.enable')
        cdp.send('CSS.enable')
        root = cdp.send('DOM.getDocument')['root']['nodeId']
        out = {}
        for key, sel in selectors.items():
            node = cdp.send('DOM.querySelector', {'nodeId': root, 'selector': sel})['nodeId']
            out[key] = sorted({f['familyName'] for f in cdp.send('CSS.getPlatformFontsForNode', {'nodeId': node})['fonts']})
        cdp.detach()
        self.used_fonts[theme] = out

    def shot(self, name, clip=None, caret='hide', note=None):
        p = self.page
        p.mouse.move(600, 2)  # Empty page margin: no hover state on any control.
        p.wait_for_timeout(60)
        path = OUT / f'{name}.png'
        args = {'path': str(path), 'animations': 'disabled', 'caret': caret, 'scale': 'device'}
        if clip:
            args['clip'] = dict(zip(('x', 'y', 'width', 'height'), clip))
        p.screenshot(**args)
        view = p.viewport_size
        with Image.open(path) as im:
            dims = list(im.size)
        self.shots.append({'file': path.name, 'css_viewport': [view['width'], view['height']], 'device_scale_factor': self.dsf,
                           'clip_css': clip, 'pixels': dims, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), **({'note': note} if note else {})})
        print(f'{name}: {dims[0]}x{dims[1]} at DSF {self.dsf}', flush=True)


def shoot(s, before):
    # Browse: every row in one tall frame.
    s.open(3)
    s.fit_height()
    s.rects('history', main='#main', rows='.history-row')
    s.fonts('Mariyaholic-98', heading='#page-title', row_title='.row-title', row_time='.row-time')
    s.layout['history']['last_row'] = list(s.page.evaluate("(()=>{const r=[...document.querySelectorAll('.history-row')].at(-1).getBoundingClientRect();return [r.x,r.y,r.width,r.height]})()"))
    s.shot('03-history', note='Window grown to the document height; no scrolling')

    # Search: empty box, the five typed states, then the real filtered result.
    s.page.set_viewport_size({'width': WIDTH, 'height': SEARCH_HEIGHT})
    s.page.wait_for_timeout(100)
    search = s.page.get_by_role('searchbox')
    search.focus()
    s.rects('search', searchbox='.searchbox', count='.result-count')
    s.shot('04-search-0-empty', caret='initial')
    for i, letter in enumerate('train', 1):
        s.page.keyboard.type(letter)
        s.shot(f'04-search-{i}-{"train"[:i]}', caret='initial')
    s.page.keyboard.press('Enter')
    expect(s.page.locator('.history-row')).to_have_count(1)
    expect(s.page.locator('.result-count')).to_have_text('1 items')
    s.rects('result', searchbox='.searchbox', count='.result-count', day='.day-heading', row='.history-row', pagination='.pagination')
    s.shot('05-search-result', note='Enter submitted the search; the backend returned one item')

    # Close-up: the same result at a density that survives a tight crop.
    s.open(5, WIDTH, SEARCH_HEIGHT)
    s.page.get_by_role('searchbox').fill('train')
    s.page.keyboard.press('Enter')
    expect(s.page.locator('.history-row')).to_have_count(1)
    s.rects('closeup', day='.day-heading', row='.history-row', title='.row-title', meta='.row-meta')
    s.shot('05-search-result-close')

    # Details and openings: the whole sheet in one tall frame.
    s.open(5)
    s.page.get_by_role('searchbox').fill('train')
    s.page.keyboard.press('Enter')
    expect(s.page.locator('.history-row')).to_have_count(1)
    s.page.locator('.history-row').click()
    expect(s.page.locator('#detail-title')).to_have_text('Building a miniature night train')
    s.fit_height('#detail')
    s.rects('details', sheet='#detail', title='#detail-title', rows='.detail-list', actions='.detail-actions', visits='.visits', caveat='.visits .fine-print', visit='.visit')
    dl = s.page.locator('.detail-list > div')
    s.layout['details']['row_duration'] = [round(v, 2) for v in dl.nth(2).bounding_box().values()]
    s.layout['details']['row_openings'] = [round(v, 2) for v in dl.nth(4).bounding_box().values()]
    s.shot('06-details', note='Details and Recent openings in one sheet; window grown so the sheet does not scroll')

    # Tag editor. Typed, never saved.
    s.page.get_by_role('button', name='Edit metadata and tags', exact=True).click()
    expect(s.page.locator('#edit-form')).to_be_visible()
    s.page.get_by_label('Type', exact=True).fill('craft')
    s.page.set_viewport_size({'width': WIDTH, 'height': HEIGHT})
    s.page.wait_for_timeout(100)
    tags = s.page.get_by_label('Tags, one per line', exact=True)
    tags.focus()
    s.rects('editor', sheet='#detail', tags='textarea[name="tags"]', save='#edit-form button[type="submit"]')
    s.shot('08-edit-tags-0-empty', caret='initial', note='Unsaved preview; Save changes is never pressed')
    s.page.keyboard.type('miniatures')
    s.shot('08-edit-tags-1-miniatures', caret='initial', note='Unsaved preview; Save changes is never pressed')
    s.page.keyboard.press('Enter')
    s.page.keyboard.type('night train')
    s.shot('08-edit-tags-2-night-train', caret='initial', note='Unsaved preview; Save changes is never pressed')
    s.page.locator('#detail [data-action="close"]').click()
    expect(s.page.locator('#detail')).not_to_be_visible()
    assert catalogue_state(s.env) == before, 'the editor changed the catalogue'

    # Sources: nothing ticked, one, two, then the confirmation, which is cancelled.
    # The window is shorter than v2's so the centred dialog clears the Automatic collection box.
    s.open(3, WIDTH, SOURCES_HEIGHT)
    s.page.locator('nav [data-page="sources"]').click()
    expect(s.page.get_by_role('checkbox')).to_have_count(2)
    s.page.evaluate('document.activeElement.blur()')
    s.rects('sources', title='.section-title', rows='.source-row', actions='.source-actions', automation='.automation')
    s.layout['sources']['row_edge'] = [round(v, 2) for v in s.page.locator('.source-row').nth(1).bounding_box().values()]
    s.shot('01-sources-0')
    s.page.get_by_role('checkbox', name='Include chrome Demo Chrome', exact=True).check()
    s.shot('01-sources-1')
    s.page.get_by_role('checkbox', name='Include edge Demo Edge', exact=True).check()
    s.shot('01-sources-2')
    s.page.get_by_role('button', name='Collect selected', exact=True).click()
    expect(s.page.locator('#confirmation')).to_be_visible()
    s.rects('confirmation', dialog='#confirmation', rows='.source-row', section='.section-title', automation='.automation')
    s.layout['confirmation']['row_edge'] = [round(v, 2) for v in s.page.locator('.source-row').nth(1).bounding_box().values()]
    s.shot('02-confirmation', note='Dialog cancelled; collection came from the omoi collect CLI run')
    s.page.get_by_role('button', name='Cancel', exact=True).click()
    assert catalogue_state(s.env) == before, 'the dialog changed the catalogue'

    # Themes: clipped above the Catalogue section so no filesystem path is ever captured.
    s.page.locator('nav [data-page="settings"]').click()
    expect(s.page.get_by_role('heading', name='Appearance', exact=True)).to_be_visible()
    s.page.evaluate('document.activeElement.blur()')
    clip = settings_clip(s)
    s.rects('settings', gallery='.theme-gallery')
    s.shot('09-appearance', clip=clip, note='Clipped above the Catalogue section')
    s.page.get_by_role('button', name='Confluence', exact=True).click()
    s.page.wait_for_timeout(100)
    clip = settings_clip(s)
    s.page.evaluate('document.activeElement.blur()')
    s.rects('settings_light', gallery='.theme-gallery')
    s.shot('09-appearance-confluence', clip=clip, note='After clicking the Confluence theme; clipped above the Catalogue section')

    # Keep browsing in the light theme: every row, as for Browse.
    s.page.locator('nav [data-page="history"]').click()
    expect(s.page.locator('.history-row')).to_have_count(8)
    s.page.evaluate('document.activeElement.blur()')
    s.fit_height()
    s.rects('confluence', main='#main', rows='.history-row')
    s.fonts('Confluence', heading='#page-title', row_title='.row-title', row_time='.row-time')
    s.shot('10-confluence', note='Window grown to the document height; no scrolling')
    s.close()


def settings_clip(s):
    gallery = s.page.locator('.theme-gallery').bounding_box()
    view = s.page.viewport_size
    bottom = gallery['y'] + gallery['height'] + 12
    left = s.page.locator('#main').bounding_box()['x']
    return [left, 0, view['width'] - left, round(bottom)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=WORK.parent / 'target' / 'release' / 'omoi')
    args = parser.parse_args()
    binary = args.binary.resolve()
    env = fresh_sandbox()
    write_histories(env)
    roots_checked = check_paths(binary, env)
    collect = json.loads(run_omoi(binary, env, 'collect'))
    before = catalogue_state(env)
    assert before['items'] == 8 and before['browser_openings'] == 8 and before['tags'] == 0, before
    OUT.mkdir(exist_ok=True)
    for old in OUT.glob('*.png'):
        old.unlink()
    proc, url = launch_ui(binary, env)
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(headless=True, args=['--font-render-hinting=none', '--disable-lcd-text'])
            version = browser.version
            s = Shooter(browser, url)
            s.env = env
            shoot(s, before)
            browser.close()
    finally:
        proc.terminate()
        proc.wait(timeout=10)
    assert not s.errors and not s.external, (s.errors, s.external)
    after = catalogue_state(env)
    assert after == before, (before, after)
    provenance = {
        'repository': 'https://github.com/mariyaholic/omoi',
        'commit': COMMIT,
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'capture_method': 'Playwright screenshots of the connected Rust browser UI (omoi ui) in headless Chromium; genuine app, synthetic browser histories',
        'browser': f'Chromium {version} headless, --font-render-hinting=none --disable-lcd-text, prefers-reduced-motion, timezone UTC, locale en-US',
        'native_windows_shell': False,
        'personal_history_used': False,
        'sandbox': {'env': ['HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'OMOI_HOME'], 'all_under': 'work/sandbox', 'omoi_paths_checked': roots_checked,
                    'collection': 'omoi collect on two fake Chromium History files: Demo Chrome and Demo Edge'},
        'collect_report': {k: collect.get(k) for k in ('profiles_found', 'profiles_failed', 'items_inserted', 'views_inserted') if k in collect},
        'catalogue_items': after['items'],
        'browser_openings': after['browser_openings'],
        'editor_saved': False,
        'catalogue_after_capture': after,
        'external_requests': s.external,
        'javascript_errors': s.errors,
        'titles_read_from_v2_captures': [c[0] for c in CATALOGUE[:4]],
        'titles_newly_authored': [c[0] for c in CATALOGUE[4:]],
        'fonts_used_on_capture_machine': s.used_fonts,
        'layout_css': s.layout,
        'shots': s.shots,
    }
    (ROOT / 'source' / 'capture-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n', encoding='utf-8')
    print('CAPTURE COMPLETE', len(s.shots), 'shots', after, flush=True)


if __name__ == '__main__':
    main()
