# omoi

Local, low-overhead watch history for the PC: reads browser history (Chrome, Edge, Brave,
Firefox) into one SQLite file, with time spent, cataloguing and recommendations planned.

Status: M0 in progress. Reader contracts, configuration, database and profile-discovery foundations are implemented; the console command remains a placeholder. See `PROGRESS.md` for current verification and remaining work, and `IMPLEMENTATION.md` for the continuing mandate.

- Design: `docs/watch-history-architecture.md` (§0 holds Maria's decisions)
- Review: `docs/omoi-design-review.md`
- M0 build contract (units U1–U14 with acceptance tests): `docs/omoi-architecture-detail.md`
