# Implementation learnings

- 2026-11-03: D: is a USB HDD on this machine. A pilot resource run overlapped Rust linking and saw a 90-second full-import wall-time outlier despite a roughly 11 MB collector peak. Complete compilation before acceptance timing, report storage and competing-load conditions, and keep CPU, wall time, copying and memory separate. Pilot raw evidence is retained locally at `D:\mariyaholic-build\omoi\measure-final-20261103\report.json`; the subsequent acceptance report uses the final executable without concurrent local compilation.
