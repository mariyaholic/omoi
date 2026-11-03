# Scripts

| Script | Purpose | Owner |
|---|---|---|
| measure.py | Windows synthetic collector resource measurements | codex.impl |
| smoke.py | CLI release journey with generated browser profiles | codex.impl |
| verify_prototype.py | Fictional-data design study interaction and screenshot checks | codex.impl |
| verify_ui.py | Connected UI journey, request boundaries, five themes, axe, real zoom and browser/server measurements | codex.impl |
| verify_playback.py | Isolated listener authentication/restart, full inbox and optional real-time hour load | codex.impl |
| verify_schedule.py | Explicit isolated Windows task install/run/remove and registered settings | codex.impl |
| verify_manager.py | Shipped userscript through a real manager in disposable headless Chromium | codex.impl |
| verify_export.py | CSV/JSON correctness and lifetime memory/CPU on an existing synthetic 100k-view catalogue | codex.impl |

Run each with `--help` for inputs. UI scripts require Python Playwright and installed Edge; manager/zoom need bundled Chromium, and optional axe auditing needs a local axe-core script. The app itself has no Python or Playwright dependency. `examples/retained_week.rs` is a separate consent-gated local-history acceptance tool; never run it in CI or commit its personal aggregate receipt.
