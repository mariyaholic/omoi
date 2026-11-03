"""Test the shipped userscript through an unpacked manager in disposable headless Chromium.

Requires Playwright's Chromium and an official Tampermonkey MV3 package extracted by
the operator. Uses a local synthetic video document at a matched platform URL, not
an authenticated platform player. External network uses a deliberately dead proxy.
"""
import argparse
from contextlib import closing
import json
import os
from pathlib import Path
import queue
import socket
import sqlite3
import subprocess
import tempfile
import threading

from playwright.sync_api import sync_playwright, expect


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--extension-dir',required=True,type=Path)
    parser.add_argument('--out',required=True,type=Path)
    parser.add_argument('--public-youtube',action='store_true',help='Use one public YouTube video anonymously instead of the offline synthetic player')
    args=parser.parse_args()
    extension=args.extension_dir.resolve()
    manifest=json.loads((extension/'manifest.json').read_text(encoding='utf-8'))
    assert manifest['manifest_version']==3
    with tempfile.TemporaryDirectory(prefix='omoi-manager-') as directory:
        root=Path(directory)
        home=root/'catalogue'
        home.mkdir()
        with closing(socket.socket()) as probe:
            probe.bind(('127.0.0.1',0))
            port=probe.getsockname()[1]
        (home/'config.toml').write_text(f'browsers=[]\nlistener_port={port}\n',encoding='utf-8')
        env=os.environ|{'OMOI_HOME':str(home)}
        for key in ('LOCALAPPDATA','APPDATA','HOME','XDG_CONFIG_HOME','XDG_DATA_HOME'):
            env[key]=str(root/'empty')
        binary=str(args.binary.resolve())

        def start():
            proc=subprocess.Popen([binary,'listen'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,creationflags=subprocess.CREATE_NO_WINDOW if os.name=='nt' else 0)
            output=queue.Queue()
            threading.Thread(target=lambda:output.put(proc.stdout.readline()),daemon=True).start()
            try:
                assert output.get(timeout=30).startswith('Playback listener:')
                return proc
            except BaseException:
                proc.kill()
                proc.wait()
                raise

        listener=start()
        try:
            token=(home/'listener/token').read_text()
            with sync_playwright() as playwright:
                context=playwright.chromium.launch_persistent_context(str(root/'browser'),channel='chromium',headless=True,
                    proxy={'server':'http://127.0.0.1:9','bypass':'127.0.0.1,localhost'+(',*.youtube.com,*.googlevideo.com,*.ytimg.com,*.ggpht.com' if args.public_youtube else '')},
                    args=[f'--disable-extensions-except={extension}',f'--load-extension={extension}'])
                try:
                    expect.set_options(timeout=30000)
                    worker=context.service_workers[0] if context.service_workers else context.wait_for_event('serviceworker')
                    extension_id=worker.url.split('/')[2]
                    settings=context.new_page()
                    settings.goto(f'chrome://extensions/?id={extension_id}')
                    toggle=settings.locator('#allow-user-scripts cr-toggle')
                    expect(toggle).to_be_visible()
                    if toggle.get_attribute('checked') is None:
                        toggle.click()
                    settings.goto(f'chrome-extension://{extension_id}/options.html')
                    expect(settings.get_by_text('Settings',exact=True)).to_be_visible()
                    settings.get_by_text('Settings',exact=True).click()
                    statistics=settings.locator('input[name="Anonymous statistics"]')
                    expect(statistics).to_be_visible()
                    statistics.uncheck()
                    settings.get_by_text('Utilities',exact=True).click()
                    script=Path(__file__).resolve().parents[1]/'userscripts/omoi.user.js'
                    with context.expect_page() as install_event:
                        settings.locator('input[type="file"]').set_input_files(str(script))
                    install=install_event.value
                    expect(install.get_by_role('button',name='Install',exact=True)).to_be_visible()
                    install.get_by_role('button',name='Install',exact=True).click()
                    video=context.new_page()
                    fixture='<title>Omoi synthetic manager acceptance</title><video muted controls></video><canvas width="64" height="64"></canvas>'
                    if args.public_youtube:
                        video.goto('https://www.youtube.com/watch?v=jNQXAC9IVRw',wait_until='domcontentloaded')
                        reject=video.get_by_role('button',name='Reject all',exact=True)
                        if reject.count():
                            reject.first.click()
                        expect(video.locator('video').first).to_be_attached()
                    else:
                        video.route('https://www.youtube.com/**',lambda route:route.fulfill(content_type='text/html',body=fixture))
                        video.goto('https://www.youtube.com/watch?v=abcdefghijk')
                    video.wait_for_timeout(1500)
                    tab_id=worker.evaluate("async()=>{const tabs=await chrome.tabs.query({});return tabs.find(t=>t.url.includes('youtube.com/watch')).id}")
                    popup=context.new_page()
                    popup.goto(f'chrome-extension://{extension_id}/action.html#{tab_id}')
                    video.bring_to_front()
                    popup.reload()
                    expect(popup.get_by_text('Pair Omoi',exact=True)).to_be_visible()
                    prompts=[]

                    def dialog(d):
                        prompts.append(d.type)
                        if 'token file' in d.message:
                            d.accept(token)
                        elif 'Listener port' in d.message:
                            d.accept(str(port))
                        else:
                            d.dismiss()
                            raise AssertionError('Unexpected pairing dialog')

                    context.on('dialog',dialog)
                    popup.get_by_text('Pair Omoi',exact=True).click()
                    video.wait_for_timeout(500)
                    assert len(prompts)==2,prompts
                    # Real HTML video playback from a generated canvas stream; no mocked GM APIs.
                    if args.public_youtube:
                        video.evaluate("async()=>{const v=document.querySelector('video');v.muted=true;await v.play()}")
                    else:
                        video.evaluate("async()=>{const canvas=document.querySelector('canvas'),ctx=canvas.getContext('2d'),v=document.querySelector('video');window.fixtureTimer=setInterval(()=>{ctx.fillStyle='#456789';ctx.fillRect(0,0,64,64)},100);v.srcObject=canvas.captureStream(10);await v.play()}")
                    video.wait_for_timeout(12000)
                    video.evaluate("document.querySelector('video').pause()")
                    video.wait_for_timeout(1000)
                    with closing(sqlite3.connect(home/'history.sqlite')) as connection:
                        events=[json.loads(row[0])['event'] for row in connection.execute('SELECT body FROM raw_events ORDER BY id')]
                    assert len(events)>=3,{'accepted_events':len(events)}
                    assert max(e['cumulative_played_ms'] for e in events)>=8000
                    assert token not in video.locator('html').inner_html()
                    collected=subprocess.run([binary,'collect'],env=env,capture_output=True,text=True,timeout=30)
                    assert collected.returncode==0,collected.stderr
                    with closing(sqlite3.connect(home/'history.sqlite')) as connection:
                        played=connection.execute('SELECT sum(played_s) FROM sessions').fetchone()[0]
                        pending=connection.execute('SELECT count(*) FROM raw_events').fetchone()[0]
                        assert played>=8 and pending==0,(played,pending)
                    report={'result':'pass','manager_version':manifest['version'],'browser':context.browser.version,
                        'accepted_events':len(events),'session_played_seconds':played,'pairing_prompts':len(prompts),
                        'checks':['unmodified shipped script installed through manager UI','pairing through real menu and protected GM storage','real HTML video events','real GM_xmlhttpRequest accepted by strict listener','token absent from page HTML','transactional session processing'],
                        'scope':('one public YouTube video in a disposable anonymous headless browser; only YouTube/media hosts and loopback permitted; no normal browser profile or credentials used' if args.public_youtube else 'disposable headless Chromium; unpacked official manager package; synthetic document at matched YouTube URL; external network blocked by dead proxy; no normal browser profile or authenticated platform used')}
                finally:
                    context.close()
        finally:
            listener.terminate()
            listener.wait(timeout=10)
    args.out.write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    main()
