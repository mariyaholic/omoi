"""Exercise the connected UI on isolated synthetic data; never read personal history.

Requires Python Playwright and installed Edge. Saves screenshots and a JSON receipt.
"""
import argparse
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
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
                with page.expect_download() as downloaded:
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
                            if width != 320:
                                page.screenshot(path=str(args.out / f"{screen}-{skin}-{width}.png"))
                        page.locator('nav [data-page="history"]').click()
                        expect(page.locator(".history-row")).to_have_count(50)
                        page.locator(".history-row").first.click()
                        expect(page.locator("#detail")).to_be_visible()
                        assert not page.locator("#detail").evaluate("e=>e.scrollWidth>e.clientWidth"), f"{skin} detail overflow"
                        page.keyboard.press("Escape")
                page.goto("about:blank")
                page.goto(launch_url)
                expect(page.locator("body")).to_have_attribute("data-skin", "oak")
                expect(page.locator(".history-row")).to_have_count(50)
                page.reload()
                expect(page.locator("#message")).to_contain_text("Open the launch URL")
                assert not errors, errors
                assert not external, external
                browser.close()
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
                             "server_idle_cpu_ms_over_3s": cpu() - before, "browser_memory": "not measured"}
            report = {"result": "pass", "fixture_items_after_delete": 10000,
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


if __name__ == "__main__":
    main()
