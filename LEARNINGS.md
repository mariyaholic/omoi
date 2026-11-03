# Implementation learnings

- 2026-11-03: Task Scheduler COM receives a UTF-16 BSTR. Declaring UTF-8 in that XML causes registration to fail even when the intermediate file decodes correctly. Omit the encoding declaration and verify actual isolated registration, not only XML strings. Token ACL setup uses the .NET access-control API directly because PowerShell Security module autoload can fail under isolated profile environment roots.

- 2026-11-03: CSS custom-property aliases declared on `:root` can inherit already-resolved colors into a theme applied on `body`. Re-declare the aliases at the theme scope; test the computed body background as well as the theme attribute. The Omoi Paper prototype initially changed inputs/sidebar while leaving the main surface dark, which screenshots exposed.

- 2026-11-03: D: is a USB HDD on this machine. A pilot resource run overlapped Rust linking and saw a 90-second full-import wall-time outlier despite a roughly 11 MB collector peak. Complete compilation before acceptance timing, report storage and competing-load conditions, and keep CPU, wall time, copying and memory separate. Pilot raw evidence is retained locally at `D:\mariyaholic-build\omoi\measure-final-20261103\report.json`; the subsequent acceptance report uses the final executable without concurrent local compilation.
# Connected catalogue UI, 2026-11-03

SQLite statistics from a one-row catalogue can make a correlated-rowid LEFT JOIN scan the entire views table per item after a large import. The 10k browser journey reproduced the stall; a standalone query plan confirmed it. Use indexed scalar lookups for the latest view and keep a growth-after-ANALYZE regression. Also park idle browser preconnections instead of blocking on each socket before accepting actual requests.
