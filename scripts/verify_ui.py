"""Exercise the connected UI on isolated synthetic data; never read personal history.

Requires Python Playwright and installed Edge. Saves screenshots and a JSON receipt.
"""
import argparse
import base64
import ctypes
from contextlib import closing
import http.client
import json
import os
from pathlib import Path
import queue
import socket
import sqlite3
import statistics
import subprocess
import tempfile
import threading
import time
from urllib.parse import urlsplit, parse_qs

from playwright.sync_api import sync_playwright, expect


def launch(binary, env, idle=120, port=0):
    proc = subprocess.Popen([str(binary), "ui", "--port", str(port), "--idle-seconds", str(idle)], env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
    lines = queue.Queue()
    threading.Thread(target=lambda: lines.put(proc.stdout.readline()), daemon=True).start()
    try:
        url = lines.get(timeout=30).strip()
        assert url.startswith("http://127.0.0.1:"), "UI did not emit a launch URL"
        return proc, url
    except BaseException:
        proc.kill()
        proc.wait()
        raise


def browser_memory(browser):
    if os.name!='nt':
        return {}
    from ctypes import wintypes
    from measure import Memory

    class MemoryEx(Memory):
        _fields_=[('private_usage',ctypes.c_size_t)]

    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    psapi=ctypes.WinDLL('psapi',use_last_error=True)
    kernel.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD]
    kernel.OpenProcess.restype=wintypes.HANDLE
    kernel.CloseHandle.argtypes=[wintypes.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes=[wintypes.HANDLE,ctypes.POINTER(MemoryEx),wintypes.DWORD]
    processes=browser.new_browser_cdp_session().send('SystemInfo.getProcessInfo')['processInfo']
    readings=[]
    for p in processes:
        handle=kernel.OpenProcess(0x410,False,int(p['id']))
        if not handle:
            continue
        try:
            memory=MemoryEx()
            memory.cb=ctypes.sizeof(memory)
            if psapi.GetProcessMemoryInfo(handle,ctypes.byref(memory),memory.cb):
                readings.append({'type':p['type'],'working_set_bytes':memory.working,'private_bytes':memory.private_usage})
        finally:
            kernel.CloseHandle(handle)
    return {'processes':readings,'reported_processes':len(processes),'measured_processes':len(readings),'sum_private_bytes':sum(p['private_bytes'] for p in readings),'scope':'isolated headless Edge processes reported by CDP; instantaneous private bytes, not normal browser incremental cost; working sets share pages'}


def verify_zoom(playwright, root, launch_url, out):
    """Use actual browser tab zoom, not CSS zoom or pinch-scale emulation."""
    extension=root/'zoom-extension'
    extension.mkdir()
    (extension/'manifest.json').write_text(json.dumps({'manifest_version':3,'name':'Omoi isolated zoom test','version':'1.0','host_permissions':['http://127.0.0.1/*'],'background':{'service_worker':'worker.js'}}),encoding='utf-8')
    (extension/'worker.js').write_text('chrome.runtime.onInstalled.addListener(()=>{});',encoding='utf-8')
    context=playwright.chromium.launch_persistent_context(str(root/'zoom-browser'),channel='chromium',headless=True,
        no_viewport=True,args=['--window-size=1280,960','--force-device-scale-factor=1',f'--disable-extensions-except={extension}',f'--load-extension={extension}'])
    checks=[]
    try:
        worker=context.service_workers[0] if context.service_workers else context.wait_for_event('serviceworker')
        page=context.new_page()
        page.goto(launch_url)
        page.bring_to_front()
        expect(page.locator('.history-row')).to_have_count(50)
        # Windows window decorations consume part of --window-size. Set the client width.
        cdp=context.new_cdp_session(page)
        window=cdp.send('Browser.getWindowForTarget')
        cdp.send('Browser.setWindowBounds',{'windowId':window['windowId'],'bounds':{'width':window['bounds']['width']+1280-page.evaluate('innerWidth')}})
        page.wait_for_function('innerWidth===1280')
        assert page.evaluate('innerWidth')==1280,page.evaluate('({width:innerWidth,dpr:devicePixelRatio})')
        tab=worker.evaluate("async()=>{const tabs=await chrome.tabs.query({url:'http://127.0.0.1/*'});return tabs[0].id}")
        for name in ('Uika','Confluence','Panel','Mariyaholic-98','Argonaut Oak'):
            worker.evaluate('([tab,zoom])=>chrome.tabs.setZoom(tab,zoom)',[tab,1])
            page.locator('nav [data-page="settings"]').click()
            page.get_by_role('button',name=name,exact=True).click()
            for zoom in (2,4):
                worker.evaluate('([tab,zoom])=>chrome.tabs.setZoom(tab,zoom)',[tab,zoom])
                page.wait_for_function('(width)=>Math.abs(innerWidth-width)<=2',arg=1280/zoom)
                for screen in ('history','suggestions','sources','settings'):
                    page.locator(f'nav [data-page="{screen}"]').click()
                    expect(page.locator('#content')).to_have_attribute('aria-busy','false')
                    assert not page.evaluate('document.documentElement.scrollWidth>innerWidth'),(name,zoom,screen)
                    checks.append({'theme':name,'zoom':zoom,'screen':screen,'css_viewport_width':page.evaluate('innerWidth')})
                page.locator('nav [data-page="history"]').click()
                page.locator('.history-row').first.click()
                page.get_by_role('button',name='Edit metadata and tags',exact=True).click()
                assert not page.locator('#detail').evaluate('e=>e.scrollWidth>e.clientWidth'),(name,zoom,'edit-dialog')
                page.get_by_label('Title',exact=True).fill('Zoom fixture')
                page.get_by_role('button',name='Save changes',exact=True).click()
                expect(page.locator('#message')).to_have_text('Item saved.')
                if zoom==4:
                    page.locator('#page-title').scroll_into_view_if_needed()
                    expect(page.locator('#page-title')).to_be_in_viewport()
                    page.bring_to_front()
                    page.evaluate('()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))')
                    capture=cdp.send('Page.captureScreenshot',{'format':'png','fromSurface':True,'captureBeyondViewport':False})
                    (out/f'zoom-400-{name.lower()}.png').write_bytes(base64.b64decode(capture['data']))
                checks.append({'theme':name,'zoom':zoom,'screen':'edit-dialog','action':'saved'})
        return {'result':'pass','method':'chrome.tabs.setZoom in disposable headless Chromium; actual 200% and 400% browser zoom','checks':checks}
    finally:
        context.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument('--axe-file',type=Path,help='Optional local axe-core script for WCAG 2 A/AA and 2.1 AA audit')
    parser.add_argument('--zoom',action='store_true',help='Also test actual 200/400 percent browser zoom using bundled Chromium')
    args = parser.parse_args()
    binary = args.binary.resolve()
    audits=[]
    browser_resources={}
    zoom_report=None
    axe_source=args.axe_file.read_text(encoding='utf-8') if args.axe_file else None
    args.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="omoi-ui-") as temporary:
        root = Path(temporary)
        home = root / "home"
        home.mkdir()
        env = os.environ.copy()
        for key in ("LOCALAPPDATA", "APPDATA", "HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME"):
            env[key] = str(root / "empty-roots")
        env["OMOI_HOME"] = str(home)
        profile_paths = []
        for i, name in enumerate(("Selected", "Unselected")):
            profile = root / name
            profile.mkdir()
            history = profile / "History"
            with closing(sqlite3.connect(history)) as c:
                c.executescript("CREATE TABLE urls(id INTEGER PRIMARY KEY,url TEXT,title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY,url INTEGER,visit_time INTEGER,visit_duration INTEGER);")
                c.execute("INSERT INTO urls VALUES(1,?,?)", (f"https://www.youtube.com/watch?v={i+1:011}", name + " video"))
                c.execute("INSERT INTO visits VALUES(1,1,13300000000000000,10000000)")
                c.commit()
            profile_paths.append(f"edge:{history}")
        (home / "config.toml").write_text("browsers=[\"edge\"]\nprofile_paths=" + json.dumps(profile_paths), encoding="utf-8")
        proc, launch_url = launch(binary, env)
        parsed = urlsplit(launch_url)
        base = f"http://{parsed.netloc}"
        token = parse_qs(parsed.fragment)["token"][0]

        def api(path, method="GET", body=None, headers=None):
            h = {"X-Omoi-Token": token}
            if method == "POST":
                h.update({"Origin": base, "Content-Type": "application/json"})
            if headers is not None:
                h.update(headers)
            conn = http.client.HTTPConnection("127.0.0.1", parsed.port, timeout=10)
            conn.request(method, "/api/" + path, json.dumps(body) if body is not None else None, h)
            response = conn.getresponse()
            status = response.status
            data = json.loads(response.read())
            conn.close()
            return status, data

        try:
            assert api("history")[1]["total"] == 0
            assert api("history", headers={"X-Omoi-Token": ""})[0] == 403
            assert api("history", headers={"Origin": "https://evil.test"})[0] == 403
            assert api("history", headers={"Host": "evil.test"})[0] == 403
            assert api("delete", "POST", {"id": 1}, {"Origin": "null"})[0] == 403
            assert api("delete", "POST", {"id": 1}, {"Content-Type": "text/plain"})[0] == 403
            assert api("collect", "POST", {"profiles": []})[0] == 400
            parked = [socket.create_connection(("127.0.0.1", parsed.port), timeout=10) for _ in range(4)]
            try:
                start = time.perf_counter()
                assert api("info")[0] == 200
                assert time.perf_counter() - start < 1, "Idle speculative sockets blocked the API"
            finally:
                for connection in parked:
                    connection.close()
            for header in (f"Host: {parsed.netloc}\r\nHost: evil.test", f"Host: {parsed.netloc}\r\nContent-Length: 999999", f"Host: {parsed.netloc}\r\nTransfer-Encoding: chunked"):
                with socket.create_connection(("127.0.0.1", parsed.port), timeout=10) as connection:
                    connection.sendall(f"POST /api/delete HTTP/1.1\r\n{header}\r\n\r\n".encode())
                    status_line = b""
                    while not status_line.endswith(b"\r\n"):
                        chunk = connection.recv(1)
                        assert chunk, "response ended before its status line"
                        status_line += chunk
                    assert b"400 Bad Request" in status_line, status_line
            assert api("info")[0] == 200
            collision = subprocess.run([str(binary), "ui", "--port", str(parsed.port)], env=env, capture_output=True, timeout=15)
            assert collision.returncode != 0
            errors, external, responses = [], [], []
            expect.set_options(timeout=15000)
            with sync_playwright() as pw:
                browser = pw.chromium.launch(channel="msedge", headless=True)
                page = browser.new_page(viewport={"width": 1440, "height": 1000})
                page.wait_for_timeout(2000)
                before_app=browser_memory(browser)
                page.set_default_timeout(10000)
                page.on("pageerror", lambda error: errors.append(str(error)))
                page.on("response", lambda r: responses.append((urlsplit(r.url).path, r.status)))
                page.on("request", lambda r: external.append(r.url) if not r.url.startswith((base, "data:")) else None)
                page.goto(launch_url)
                expect(page.get_by_role("heading", name="No history yet", exact=True)).to_be_visible()
                assert "token" not in page.url
                page.get_by_role("button", name="Select sources", exact=True).click()
                expect(page.get_by_role("checkbox")).to_have_count(2)
                page.get_by_role("checkbox", name="Include edge Selected", exact=True).check()
                page.get_by_role("button", name="Collect selected", exact=True).click()
                page.get_by_role("dialog").get_by_role("button", name="Collect", exact=True).click()
                expect(page.locator("#message")).to_contain_text("1 items added", timeout=60000)
                with closing(sqlite3.connect(home / "history.sqlite")) as c:
                    assert c.execute("SELECT title FROM items").fetchall() == [("Selected video",)]
                    c.executemany("INSERT INTO items(platform,external_id,title,channel,kind,duration_s) VALUES('youtube',?,?,?,?,?)",
                                  [(f"{i+100:011}", f"Catalogue video {i:05}", "Test channel", "video", 600) for i in range(10000)])
                    c.execute("UPDATE items SET title=? WHERE id=2", ('<img src=x onerror="alert(1)">',))
                    c.execute("INSERT INTO views(item_id,started_at,source) SELECT id,1700000000+id,'edge:fixture' FROM items WHERE id>1")
                    c.commit()
                page.locator('nav [data-page="history"]').click()
                page.get_by_role("button", name="Refresh", exact=True).click()
                try:
                    expect(page.locator(".history-row")).to_have_count(50)
                except AssertionError:
                    print(json.dumps({"message":page.locator("#message").inner_text(), "content":page.locator("#content").inner_text()[:1000], "errors":errors,"server_exit":proc.poll(),"responses":responses}), flush=True)
                    raise
                expect(page.locator(".result-count")).to_have_text("10001 items")
                page.wait_for_timeout(2000)
                before_audit=browser_memory(browser)
                first = page.locator(".history-row").first.inner_text()
                page.get_by_role("button", name="Next", exact=True).click()
                expect(page.locator(".pagination")).to_contain_text("51–100")
                assert first != page.locator(".history-row").first.inner_text()
                page.get_by_role("searchbox").fill("Catalogue video 09999")
                page.get_by_role("button", name="Search", exact=True).click()
                expect(page.locator(".history-row")).to_have_count(1)
                page.locator(".history-row").click()
                expect(page.get_by_role("dialog", name="Catalogue video 09999")).to_be_visible()
                assert page.get_by_role("link", name="Open original").get_attribute("href").startswith("https://www.youtube.com/watch?v=")
                page.get_by_role("button", name="Delete item", exact=True).click()
                page.get_by_role("button", name="Cancel", exact=True).click()
                expect(page.locator("#detail")).to_be_visible()
                page.get_by_role("button", name="Delete item", exact=True).click()
                page.locator("#confirmation").get_by_role("button", name="Delete item", exact=True).click()
                expect(page.locator("#message")).to_have_text("Item deleted.")
                assert api("history?q=Catalogue%20video%2009999")[1]["total"] == 0
                page.get_by_role("searchbox").fill("<img")
                page.get_by_label("Search mode", exact=True).select_option("literal")
                page.get_by_role("button", name="Search", exact=True).click()
                expect(page.locator(".history-row")).to_have_count(1)
                assert page.locator(".history-row img").count() == 0
                page.get_by_role("button", name="Clear", exact=True).click()
                expect(page.locator(".history-row")).to_have_count(50)
                warm = []
                page.locator('.history-row').first.click()
                page.get_by_role('button', name='Edit metadata and tags', exact=True).click()
                page.get_by_label('Title', exact=True).fill('Edited fixture')
                page.get_by_label('Type', exact=True).fill('documentary')
                page.get_by_label('Tags, one per line', exact=True).fill('Space\nScience')
                page.get_by_role('button', name='Save changes', exact=True).click()
                expect(page.locator('#message')).to_have_text('Item saved.')
                page.locator('.history-row').first.click()
                expect(page.locator('#detail-title')).to_have_text('Edited fixture')
                expect(page.locator('#detail .tags')).to_contain_text('space')
                for _ in range(12):
                    page.keyboard.press('Tab')
                    assert page.evaluate("document.querySelector('#detail').contains(document.activeElement)")
                page.keyboard.press('Escape')
                expect(page.locator('.history-row').first).to_be_focused()
                page.locator('nav [data-page="sources"]').click()
                page.get_by_role('button', name='Enable automatic collection', exact=True).click()
                page.get_by_role('button', name='Cancel', exact=True).click()
                expect(page.locator('#confirmation')).not_to_be_visible()
                page.locator('nav [data-page="settings"]').click()
                page.get_by_role('button', name='Check playback inbox', exact=True).click()
                expect(page.locator('#playback-output')).to_contain_text('0 events pending')
                page.get_by_role('button', name='Process playback', exact=True).click()
                expect(page.locator('#playback-output')).to_contain_text('0 events processed')
                page.evaluate('window.showSaveFilePicker=undefined')
                with page.expect_download(timeout=60000) as downloaded:
                    page.get_by_role('button', name='Export JSON', exact=True).click()
                exported = json.loads(Path(downloaded.value.path()).read_text(encoding='utf-8'))
                assert len(exported) == 10000
                assert any(v['title'] == 'Edited fixture' for v in exported)
                for _ in range(10):
                    page.locator('nav [data-page="settings"]').click()
                    expect(page.get_by_role("heading", name="Appearance", exact=True)).to_be_visible()
                    start = time.perf_counter()
                    page.locator('nav [data-page="history"]').click()
                    expect(page.locator(".history-row")).to_have_count(50)
                    warm.append((time.perf_counter() - start) * 1000)
                for name, skin in (("Uika", "night"), ("Confluence", "paper"), ("Panel", "panel"), ("Mariyaholic-98", "pc98"), ("Argonaut Oak", "oak")):
                    page.locator('nav [data-page="settings"]').click()
                    page.get_by_role("button", name=name, exact=True).click()
                    for width in (1440, 375, 320):
                        page.set_viewport_size({"width": width, "height": 1000 if width == 1440 else 900})
                        for screen in ("history", "suggestions", "sources", "settings"):
                            page.locator(f'nav [data-page="{screen}"]').click()
                            expect(page.locator("#content")).to_have_attribute("aria-busy", "false")
                            assert not page.evaluate("document.documentElement.scrollWidth > innerWidth"), f"{name} / {screen} overflow at {width}"
                            if axe_source and width==1440:
                                page.evaluate(axe_source)
                                audit=page.evaluate("async()=>{const r=await axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21aa']}});return {violations:r.violations.map(v=>({id:v.id,impact:v.impact,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))})),incomplete:r.incomplete.map(v=>v.id)}}")
                                audits.append({'theme':name,'screen':screen,**audit})
                            if width != 320:
                                page.screenshot(path=str(args.out / f"{screen}-{skin}-{width}.png"))
                        page.locator('nav [data-page="history"]').click()
                        expect(page.locator(".history-row")).to_have_count(50)
                        page.locator(".history-row").first.click()
                        expect(page.locator("#detail")).to_be_visible()
                        assert not page.locator("#detail").evaluate("e=>e.scrollWidth>e.clientWidth"), f"{skin} detail overflow"
                        if axe_source and width==1440:
                            page.get_by_role('button',name='Edit metadata and tags',exact=True).click()
                            audit=page.evaluate("async()=>{const r=await axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21aa']}});return {violations:r.violations.map(v=>({id:v.id,impact:v.impact,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))})),incomplete:r.incomplete.map(v=>v.id)}}")
                            audits.append({'theme':name,'screen':'edit-dialog',**audit})
                        page.keyboard.press("Escape")
                page.goto("about:blank")
                page.goto(launch_url)
                expect(page.locator("body")).to_have_attribute("data-skin", "oak")
                expect(page.locator(".history-row")).to_have_count(50)
                assert not errors, errors
                assert not external, external
                browser_resources={'before_app':before_app,'catalogue_before_audit':before_audit,'after_journey':browser_memory(browser),'axe_loaded':bool(axe_source)}
                page.reload()
                expect(page.locator("#message")).to_contain_text("Open the launch URL")
                browser.close()
                if args.zoom:
                    zoom_report=verify_zoom(pw,root,launch_url,args.out)
            latency = []
            for _ in range(20):
                start = time.perf_counter()
                status, data = api("history")
                assert status == 200 and len(data["items"]) == 50
                latency.append((time.perf_counter() - start) * 1000)
            resources = {}
            if os.name == "nt":
                from ctypes import wintypes
                from measure import Memory
                kernel = ctypes.WinDLL("kernel32", use_last_error=True)
                psapi = ctypes.WinDLL("psapi", use_last_error=True)
                kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
                psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Memory), wintypes.DWORD]

                def cpu():
                    times = [wintypes.FILETIME() for _ in range(4)]
                    assert kernel.GetProcessTimes(int(proc._handle), *(ctypes.byref(t) for t in times))
                    return sum((t.dwHighDateTime << 32) | t.dwLowDateTime for t in times[2:]) / 10000

                before = cpu()
                time.sleep(3)
                memory = Memory()
                memory.cb = ctypes.sizeof(memory)
                assert psapi.GetProcessMemoryInfo(int(proc._handle), ctypes.byref(memory), memory.cb)
                resources = {"server_working_set_bytes": memory.working, "server_peak_working_set_bytes": memory.peak_working,
                             "server_idle_cpu_ms_over_3s": cpu() - before, "browser_memory": "see browser_resources"}
            report = {"result": "failed" if any(a['violations'] for a in audits) else "pass", "fixture_items_after_delete": 10000,
                      'axe_audits':audits,'browser_resources':browser_resources,'browser_zoom':zoom_report,
                      "widths": [1440, 375, 320], "themes": ["Uika", "Confluence", "Panel", "Mariyaholic-98", "Argonaut Oak"],
                      "api_ms_median": statistics.median(latency), "api_ms_max": max(latency),
                      "warm_history_click_to_rows_ms_median": statistics.median(warm), "warm_history_click_to_rows_ms_max": max(warm),
                      "resources": resources, "javascript_errors": errors, "external_requests": external,
                      "checks": ["metadata and tag editing", "dialog keyboard containment and focus restoration", "scheduler confirmation cancellation", "playback inbox processing", "JSON download", "empty catalogue", "selected-profile collection excludes unchecked profile", "10k item pagination", "FTS/literal search", "HTML escaping", "delete cancel/confirm/FTS cascade", "all screens and details in five themes", "appearance persistence", "token removed from URL", "unauthenticated and foreign Host/Origin rejection", "duplicate/oversized/chunked request rejection", "port collision", "idle shutdown"]}
        finally:
            proc.terminate()
            proc.wait(timeout=10)
        idle_proc, _ = launch(binary, env, idle=1)
        try:
            assert idle_proc.wait(timeout=5) == 0
        finally:
            if idle_proc.poll() is None:
                idle_proc.kill()
                idle_proc.wait()
        (args.out / "verification.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
        if any(a['violations'] for a in audits):
            raise SystemExit('Accessibility violations found; see saved receipt')


if __name__ == "__main__":
    main()
