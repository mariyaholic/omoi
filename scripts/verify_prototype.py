"""Check Omoi's static prototype and save desktop/mobile browser evidence.

Requires Python playwright and installed Edge. Serve prototype/ locally first.
"""
import argparse
import json
from pathlib import Path
from playwright.sync_api import sync_playwright, expect


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default="http://127.0.0.1:43829")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--channel", default="msedge")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as pw:
        browser = pw.chromium.launch(channel=args.channel, headless=True)
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, device_scale_factor=1)
        page.set_default_timeout(7000)
        errors = []
        external = []
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on("console", lambda message: errors.append(message.text) if message.type == "error" else None)
        page.on("request", lambda request: external.append(request.url) if not request.url.startswith((args.url, "data:", "file:")) else None)
        page.goto(args.url, wait_until="networkidle")
        expect(page.locator(".history-row")).to_have_count(18)
        page.screenshot(path=str(args.out / "history-desktop.png"))
        page.locator('[data-open="1"]').click()
        expect(page.get_by_role("dialog", name="How we find planets we cannot see")).to_be_visible()
        page.screenshot(path=str(args.out / "detail-desktop.png"))
        page.get_by_role("button", name="Save for later", exact=True).click()
        expect(page.get_by_role("button", name="Saved for later", exact=True)).to_be_visible()
        page.keyboard.press("Escape")
        expect(page.locator('[data-open="1"]')).to_be_focused()
        page.keyboard.press("/")
        expect(page.get_by_role("searchbox")).to_be_focused()
        page.get_by_role("searchbox").fill("coffee")
        expect(page.locator(".history-row")).to_have_count(1)
        page.get_by_role("searchbox").fill('<script>alert("x")</script>')
        expect(page.get_by_role("heading", name="No openings found.")).to_be_visible()
        page.get_by_role("button", name="Clear filters", exact=True).first.click()
        page.get_by_label("Platform", exact=True).select_option("Netflix")
        expect(page.locator(".history-row")).to_have_count(3)
        page.get_by_label("Date", exact=True).select_option("Today")
        expect(page.locator(".history-row")).to_have_count(1)
        page.get_by_role("button", name="Clear filters", exact=True).click()
        page.locator('[data-open="1"]').click()
        page.get_by_role("button", name="Remove from sample history").click()
        page.get_by_role("button", name="Cancel", exact=True).click()
        expect(page.locator("#detail")).to_be_visible()
        page.get_by_role("button", name="Remove from sample history").click()
        page.get_by_role("button", name="Remove sample entry", exact=True).click()
        expect(page.locator(".history-row")).to_have_count(17)
        page.get_by_role("button", name="Undo", exact=True).click()
        expect(page.locator(".history-row")).to_have_count(18)
        page.locator('nav [data-page="suggestions"]').click()
        expect(page.locator(".suggestion-card")).to_have_count(2)
        page.locator('[data-budget="15"]').click()
        expect(page.locator('[data-budget="15"]')).to_have_attribute("aria-pressed", "true")
        page.get_by_label("Similar by topic seed").select_option("5")
        expect(page.locator("#similar-results")).to_contain_text("Food")
        page.screenshot(path=str(args.out / "suggestions-desktop.png"))
        page.locator('nav [data-page="sources"]').click()
        page.get_by_role("checkbox", name="Include Microsoft Edge Work").check()
        expect(page.get_by_text("3 selected in this preview")).to_be_visible()
        page.get_by_role("button", name="Preview automatic collection").click()
        page.get_by_role("button", name="Enable in preview", exact=True).click()
        expect(page.get_by_text("Enabled in preview", exact=True)).to_be_visible()
        page.get_by_role("button", name="Disable in preview", exact=True).click()
        page.get_by_label("Preview state").select_option("partial")
        page.screenshot(path=str(args.out / "sources-desktop.png"))
        page.locator('nav [data-page="history"]').click()
        expect(page.get_by_text("Firefox is busy. Your Edge history is available.")).to_be_visible()
        page.get_by_label("Preview state").select_option("empty")
        expect(page.get_by_role("heading", name="Your history starts here.")).to_be_visible()
        page.screenshot(path=str(args.out / "empty-desktop.png"))
        page.get_by_role("button", name="Load sample library").click()
        page.locator('nav [data-page="settings"]').click()
        page.get_by_role("button", name="Paper", exact=True).click()
        expect(page.locator("body")).to_have_css("background-color", "rgb(243, 239, 228)")
        page.get_by_role("button", name="Compact", exact=True).click()
        page.locator('nav [data-page="history"]').click()
        page.screenshot(path=str(args.out / "history-paper.png"))
        page.locator('nav [data-page="settings"]').click()
        page.get_by_role("button", name="Panel", exact=True).click()
        expect(page.locator("body")).to_have_css("background-color", "rgb(18, 20, 24)")
        page.locator('nav [data-page="history"]').click()
        page.screenshot(path=str(args.out / "history-panel.png"))
        page.locator('nav [data-page="settings"]').click()
        with page.expect_download() as download:
            page.get_by_role("button", name="Export sample JSON").click()
        download_path = args.out / "sample-export.json"
        download.value.save_as(download_path)
        exported = json.loads(download_path.read_text(encoding="utf-8"))
        assert exported["fictional"] is True and len(exported["items"]) == 18
        page.get_by_role("button", name="Reset preview", exact=True).click()
        page.get_by_role("dialog").get_by_role("button", name="Reset preview", exact=True).click()
        expect(page.locator("body")).not_to_have_attribute("data-face", "sumimi")
        page.reload(wait_until="networkidle")
        expect(page.locator("body")).not_to_have_attribute("data-face", "sumimi")
        for width in (375, 320):
            page.set_viewport_size({"width": width, "height": 900})
            for name in ("history", "suggestions", "sources", "settings"):
                page.locator(f'nav [data-page="{name}"]').click()
                assert page.evaluate("scrollY") == 0, "Screen switch did not restore page top"
                overflow = page.evaluate("document.documentElement.scrollWidth > innerWidth")
                if overflow:
                    print(page.evaluate("Array.from(document.querySelectorAll('body *')).filter(e=>e.getBoundingClientRect().right>innerWidth+1).map(e=>({tag:e.tagName,cls:e.className,text:e.textContent.slice(0,60),right:e.getBoundingClientRect().right}))"))
                assert not overflow, f"Horizontal overflow: {name} at {width}px"
                if width == 375:
                    page.screenshot(path=str(args.out / f"{name}-mobile.png"))
            page.locator('nav [data-page="history"]').click()
            page.locator('[data-open="17"]').click()
            assert not page.locator("#detail").evaluate("el => el.scrollWidth > el.clientWidth"), "Detail sheet overflow"
            if width == 375:
                page.screenshot(path=str(args.out / "detail-mobile.png"))
            page.keyboard.press("Escape")
        page.emulate_media(reduced_motion="reduce")
        assert page.evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
        page.get_by_role("button", name="Refresh sample", exact=True).last.click()
        expect(page.get_by_role("status")).to_contain_text("No browser history accessed.")
        for theme in ("Paper", "Panel"):
            page.locator('nav [data-page="settings"]').click()
            page.get_by_role("button", name=theme, exact=True).click()
            for name in ("history", "suggestions", "sources", "settings"):
                page.locator(f'nav [data-page="{name}"]').click()
                assert not page.evaluate("document.documentElement.scrollWidth > innerWidth"), f"{theme} overflow in {name} at 320px"
            page.set_viewport_size({"width": 375, "height": 900})
            page.locator('nav [data-page="history"]').click()
            page.screenshot(path=str(args.out / f"history-{theme.lower()}-mobile.png"))
            page.set_viewport_size({"width": 320, "height": 900})
        file_url = (Path(__file__).resolve().parents[1] / "prototype/index.html").as_uri()
        page.goto(file_url, wait_until="networkidle")
        expect(page.locator(".history-row")).to_have_count(18)
        expect(page.locator("body")).to_have_css("background-color", "rgb(11, 9, 19)")
        assert not errors, errors
        assert not external, external
        report = {"result": "pass", "browser": browser.version, "screenshots": sorted(p.name for p in args.out.glob("*.png")), "widths": [1440, 375, 320], "javascript_errors": errors, "external_requests": external, "checks": ["search and literal hostile text", "platform/date filters", "save/remove/cancel/undo", "Escape and focus return", "search shortcut", "suggestion budget and topic selection", "profile selection", "simulated automation", "partial/empty states", "Night/Paper/Panel themes and computed colors", "density and reset on reload", "all screens at 320 and 375 pixels", "Paper/Panel all screens at 320 pixels", "long title detail", "reduced motion", "sample export and reset", "mobile refresh", "direct file loading"]}
        (args.out / "verification.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
        browser.close()


if __name__ == "__main__":
    main()
