# Cloud handoff: guided setup implementation

Updated 2026-11-03 on `feat/guided-setup`. IF1–IF7 source implementation is on this branch. Both hosted Ubuntu and Windows acceptance passed on implementation head 6699b83, including installer/registration/portable preservation checks. Two subsequent UI refresh regressions passed locally and are included in current CI. Real Edge companion, device, signing and store gates remain explicit. See [current acceptance](docs/guided-setup-acceptance.md), [setup instructions](docs/SETUP.md) and [protocol](docs/capture-protocol.md).

The original assignment follows for context. Cloud means a development checkout;
Omoi remains a local application, with no hosted catalogue or cloud ingestion.

## Start here

1. Read [the installation plan](docs/installation-friction.md), then
   [runtime state machines](docs/state-machines.md).
2. Use this documentation branch, or main after these docs merge. The code baseline
   is merge `cc530f0` (PR #4), containing audit commit `3bb38cc`. The preceding
   desktop/icon work is already merged. Do not restart the historical U1 scaffold.
3. Inspect `git status --short --branch`, the current CI workflow and toolchain.
   Preserve unrelated edits. Work on a feature branch; never push/rewrite main.
4. Continue from the implemented setup/managed listener/native companion and installer sources. Do not recreate the scaffold. Resolve the remaining acceptance gates recorded in docs/guided-setup-acceptance.md.

## Goal

A new Windows/Edge user can install Omoi, choose their history scope, optionally
enable playback capture, and see a first recorded session without a terminal,
token-file hunt, separate listener launch or manual inbox processing. Browser
installation/site-permission approval remains explicit. Userscript support remains
available while a dedicated Edge companion becomes the default distribution path.

The current app works. The current setup is manual. Preserve the existing SQLite
catalogue, schema migration guarantees, five themes, Pixel return icon and CLI.
No decorative taglines or one-line filler; show factual state and useful actions.

## First verification

From the repository root, with Rust stable, native compiler, Python and Node:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
node --test tests/userscript.test.cjs
python tests/desktop_state.py
cargo build --locked --release --bins --example synthetic
python scripts/smoke.py --bin-dir target/release
python scripts/verify_playback.py --binary target/release/omoi
```

Those last paths are for Linux/default Cargo target. On Windows use the `.exe`
suffix and the GNU setup/staging order in [.github/workflows/ci.yml](.github/workflows/ci.yml).
If `CARGO_TARGET_DIR` is set, use it in executable arguments. Windows-only checks:

```text
python scripts/verify_desktop.py --bin-dir <target>/release
python tests/scheduler_state.py
python tests/ui_state.py
```

The last command requires Playwright and installed Edge; CI currently pins
Playwright 1.62.0. `verify_desktop.py --stage-only` stages the loader for Windows
debug checks after compilation, as in CI. Linux's desktop unsupported stub is
not evidence of Windows desktop behavior. Do not assume the owner's D: paths,
Windows profile, browser, local receipts or ignored WORKSTACK exist in cloud.

## Scope and delivery

Use the plan's source map and acceptance matrix. Add cohesive regression tests
for state transitions, not tests that merely repeat implementation. Keep existing
checks enabled. Supply a reviewable implementation PR, updated setup docs, reproducible
synthetic fixtures, test results by environment and Windows package instructions.
Do not stop merely because a work package ends or a store listing is unavailable.

Use synthetic data and disposable profiles. Do not upload personal browser data,
tokens, extension storage, clipboard contents, installed program files or private
acceptance receipts. Do not publish an extension/release, acquire certificates,
spend money, force-install through policy, or change someone's real browser or
scheduled tasks as part of cloud development. Prepare those artifacts and name
the exact remaining owner action. No new cloud service, paid API or account is
required for the application.

Local verification of the audit baseline passed 85 Windows Rust tests, eight UI
transition tests, seven capture tests and release journeys. Both hosted Windows
and Ubuntu jobs passed for PR #4. An owner-assisted Edge setup later demonstrated
real capture, but does not establish clean-device installation or all providers.
Re-run relevant checks on your own changes; inherited results are baseline evidence.

## Paste into a cloud task

> Implement the installation-friction roadmap in CLOUD_HANDOFF.md and
> docs/installation-friction.md on the current Omoi code. Start with IF1 and continue
> through the independently implementable packages. Preserve local-only operation,
> schema/data, all five themes, the Pixel return icon and the existing CLI. Use
> synthetic fixtures; never import personal browser data. Make setup resumable,
> manage capture from the app, automatically materialize received playback, and
> build the Edge companion/packaging path without weakening origin checks. Keep
> unavailable device/store checks explicitly pending and continue other work.
> Deliver tested commits and a PR with exact remaining manual release steps.
