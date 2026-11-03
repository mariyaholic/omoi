# Command-line baseline

The current CLI supports manual collection, safe CSV/JSON export, database inspection, optional Windows hourly/logon scheduling and explainable local recommendation shelves. The implementation has moved beyond the earlier 27-test manual-only baseline.

Pipeline: profile discovery → sidecar-aware private snapshot → bounded Chromium/Firefox pagination → platform normalization → per-profile spool → one transaction for dedupe and checkpoints → bounded summary logging. Unchanged source files skip reading; a checkpoint conflict gets one retry; weekly maintenance uses `PRAGMA optimize`, never automatic VACUUM.

The [verification summary](docs/headless-acceptance-20261103.md) records completed headless verification. [README](README.md) gives commands and explains how to enable automation deliberately. The connected interface is documented in [UI instructions](ui/README.md).

The owner-authorized retained-week snapshot import and isolated scheduled-helper run have passed. A build does not install a task or enable collection. Further acceptance is deferred for the private development merge; the detailed follow-up is retained locally.
