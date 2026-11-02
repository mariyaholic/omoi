# Command-line baseline

The current CLI supports manual collection, safe CSV/JSON export, database inspection, optional Windows hourly/logon scheduling and explainable local recommendation shelves. The implementation has moved beyond the earlier 27-test manual-only baseline.

Pipeline: profile discovery → sidecar-aware private snapshot → bounded Chromium/Firefox pagination → platform normalization → per-profile spool → one transaction for dedupe and checkpoints → bounded summary logging. Unchanged source files skip reading; a checkpoint conflict gets one retry; weekly maintenance uses `PRAGMA optimize`, never automatic VACUUM.

The current evidence and unresolved M0 gates are maintained in [M0 acceptance](docs/m0-acceptance.md). Use that report rather than historical progress entries or design targets as evidence. [README](README.md) gives commands and explains how to enable automation deliberately. [UI direction](docs/ui-recommendations.md) describes the recommended next interface.

No private browser history or cookies were read during implementation. A build does not install a task. A passing synthetic suite does not replace the owner's retained-week import or an actual scheduled-launch smoke test. The PR remains draft until the open acceptance evidence is reviewed.
