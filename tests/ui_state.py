"""Headless transition tests for the shipped UI, with controlled API completion order.

Run: python tests/ui_state.py (requires the same Playwright/Edge as verify_ui.py).
Only synthetic data and intercepted omoi.test requests are used.
"""
import json
import os
from pathlib import Path
import unittest
from urllib.parse import urlsplit, parse_qs

from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]


class UiTransitions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.pw = sync_playwright().start()
        cls.browser = cls.pw.chromium.launch(**({"executable_path":os.environ["OMOI_TEST_BROWSER"]} if os.environ.get("OMOI_TEST_BROWSER") else {"channel":"msedge"}), headless=True)

    @classmethod
    def tearDownClass(cls):
        cls.browser.close()
        cls.pw.stop()

    def setUp(self):
        self.page = self.browser.new_page()
        self.page.set_default_timeout(3000)
        self.hold, self.pending, self.errors = set(), [], []
        self.page.on("pageerror", lambda error: self.errors.append(str(error)))
        self.page.route("**/*", self.route)
        self.page.goto("http://omoi.test/#token=" + "a" * 64)
        expect(self.page.locator(".history-row")).to_have_count(2)

    def tearDown(self):
        self.page.close()

    @staticmethod
    def item(identifier):
        return {"id": identifier, "platform": "youtube", "external_id": "abcdefghijk",
                "title": f"Item {identifier}", "channel": "Studio", "kind": "video",
                "duration_s": 600, "played_seconds": 0, "visit_count": 1, "tags": [],
                "visits": [], "last_opened": 1700000000, "source": "edge"}

    def data(self, url):
        path = url.path
        if path == "/api/info":
            return {"items": 2, "platforms": ["youtube"], "catalogue": "fixture.sqlite",
                    "config": "fixture.toml", "idle_seconds": 0}
        if path == "/api/history":
            return {"items": [self.item(1), self.item(2)], "total": 2}
        if path == "/api/item":
            return self.item(int(parse_qs(url.query)["id"][0]))
        if path == "/api/sources":
            return {"profiles": [{"id": "one", "browser": "edge", "name": "One"},
                                 {"id": "two", "browser": "edge", "name": "Two"}],
                    "excluded": 0, "failures": [], "warnings": []}
        if path == "/api/setup":
            return {"client":"unknown","transport":"stopped","data":"no_event_yet","pending":0,"client_version":"0.2.0","settings":{"generation":0,"choice":"not_chosen","step":"choose","background":False,"revision":0,"last_seen":None,"last_event":None,"last_processed":None}}
        if path == "/api/playback":
            return {"pending": 0, "dropped": 0, "port": 43821, "token_file": "fixture"}
        return {"saved": True}

    def route(self, route):
        url = urlsplit(route.request.url)
        if url.hostname != "omoi.test":
            route.abort()
        elif url.path.startswith("/api/"):
            if url.path in self.hold:
                self.pending.append(route)
            else:
                route.fulfill(json=self.data(url))
        else:
            name = url.path.lstrip("/") or "index.html"
            candidates = [ROOT / directory / name for directory in ("ui", "prototype", "assets")]
            file = next((p for p in candidates if p.is_file()), None)
            if file:
                mime = {".js": "text/javascript", ".css": "text/css", ".html": "text/html",
                        ".svg": "image/svg+xml"}.get(file.suffix, "application/octet-stream")
                route.fulfill(body=file.read_bytes(), content_type=mime)
            else:
                route.fulfill(status=404, body="")

    def resolve(self, path, index=0, data=None, status=200):
        matches = []
        for _ in range(30):
            matches = [r for r in self.pending if urlsplit(r.request.url).path == path]
            if len(matches) > index:
                break
            self.page.wait_for_timeout(10)
        self.assertGreater(len(matches), index, f"Missing pending request {path}")
        route = matches[index]
        self.pending.remove(route)
        route.fulfill(status=status, content_type="application/json",
                      body=json.dumps(self.data(urlsplit(route.request.url)) if data is None else data))
        self.page.evaluate("() => new Promise(resolve => setTimeout(resolve, 20))")

    def navigate(self, page):
        self.page.locator(f'nav [data-page="{page}"]').click()

    def test_latest_detail_request_owns_dialog(self):
        self.hold.add("/api/item")
        self.page.locator('[data-item="1"]').click()
        self.page.locator('[data-item="2"]').click()
        self.resolve("/api/item", 1)
        self.resolve("/api/item")
        expect(self.page.locator("#detail-title")).to_have_text("Item 2")

    def test_navigation_cancels_pending_detail(self):
        self.hold.add("/api/item")
        self.page.locator('[data-item="1"]').click()
        self.navigate("settings")
        self.resolve("/api/item")
        expect(self.page.locator("#detail")).not_to_be_visible()

    def start_edit_then_open_other_item(self):
        self.page.locator('[data-item="1"]').click()
        self.page.get_by_role("button", name="Edit metadata and tags").click()
        self.hold.add("/api/edit")
        self.page.get_by_role("button", name="Save changes").click()
        self.page.get_by_role("button", name="Close details").click()
        self.page.locator('[data-item="2"]').click()
        expect(self.page.locator("#detail-title")).to_have_text("Item 2")

    def test_old_save_does_not_close_new_dialog(self):
        self.start_edit_then_open_other_item()
        self.resolve("/api/edit")
        expect(self.page.locator("#detail")).to_be_visible()
        expect(self.page.locator("#detail-title")).to_have_text("Item 2")

    def test_old_save_error_does_not_target_detached_form(self):
        self.start_edit_then_open_other_item()
        self.resolve("/api/edit", data={"error": "Fixture save failed"}, status=400)
        self.assertEqual(self.errors, [])
        expect(self.page.locator("#detail-title")).to_have_text("Item 2")

    def test_playback_reply_after_navigation_is_harmless(self):
        self.navigate("settings")
        expect(self.page.get_by_role("button", name="Retry connection")).to_be_visible()
        self.hold.add("/api/setup")
        self.page.get_by_role("button", name="Retry connection").click()
        self.navigate("history")
        self.resolve("/api/setup")
        expect(self.page.locator("#message")).to_have_text("")
        expect(self.page.locator("#connection-state")).to_have_text("Connected")

    def test_stale_discovery_does_not_clear_current_selection(self):
        self.hold.add("/api/sources")
        self.navigate("sources")
        self.navigate("history")
        self.navigate("sources")
        self.resolve("/api/sources", 1)
        self.page.locator('[data-profile="two"]').check()
        self.resolve("/api/sources", data={"profiles": [], "excluded": 0, "failures": [], "warnings": []})
        self.page.get_by_role("button", name="Collect selected", exact=True).click()
        expect(self.page.locator("#confirmation")).to_be_visible()

    def test_shrinking_catalogue_recovers_from_an_empty_later_page(self):
        self.hold.add("/api/history")
        self.navigate("history")
        self.resolve("/api/history", data={"items": [self.item(1), self.item(2)], "total": 51})
        self.page.get_by_role("button", name="Next", exact=True).click()
        self.resolve("/api/history", data={"items": [], "total": 2})
        self.hold.remove("/api/history")
        self.resolve("/api/history")
        expect(self.page.locator(".pagination")).to_contain_text("1–2 of 2")

    def test_missing_suggestion_seed_can_be_cleared_after_error(self):
        self.page.locator('[data-item="1"]').click()
        self.hold.add("/api/suggestions")
        self.page.get_by_role("button", name="Similar by topic", exact=True).click()
        self.resolve("/api/suggestions", data={"error": "Item not found"}, status=400)
        expect(self.page.get_by_role("button", name="Clear topic selection")).to_be_visible()

    def test_setup_load_does_not_replace_new_page(self):
        self.hold.add("/api/setup")
        self.navigate("settings")
        self.navigate("sources")
        self.resolve("/api/setup")
        expect(self.page.get_by_role("button", name="Collect selected", exact=True)).to_be_visible()
        expect(self.page.locator("#capture-setup")).to_have_count(0)

    def test_history_is_available_without_capture(self):
        self.navigate("settings")
        expect(self.page.locator("#capture-status")).to_contain_text("no_event_yet")
        expect(self.page.locator("#capture-status")).to_contain_text("Client: unknown")
        self.navigate("history")
        expect(self.page.locator(".history-row")).to_have_count(2)

    def test_automatic_collection_submits_only_selected_profiles(self):
        self.navigate("sources")
        self.page.locator('[data-profile="two"]').check()
        self.page.get_by_role("button", name="Enable automatic collection").click()
        with self.page.expect_request("**/api/schedule") as request:
            self.page.locator('#confirmation [data-action="confirm"]').click()
        self.assertEqual(request.value.post_data_json, {"action":"install","profiles":["two"]})


if __name__ == "__main__":
    unittest.main()
