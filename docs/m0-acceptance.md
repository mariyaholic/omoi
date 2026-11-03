# M0 implementation record

The private development merge proceeds with the owner's acceptance deferral. The detailed acceptance matrix, historical measurements and follow-up checklist are retained locally. The [current verification summary](headless-acceptance-20261103.md) records completed work without treating deferred cases as passed.

The implemented pipeline includes profile discovery, snapshots, bounded readers, normalization, transactional dedupe/checkpoints, exports, bounded logs, maintenance and optional Windows scheduling. The original architecture contract remains in [the design document](omoi-architecture-detail.md).

## Implementation notes

Windows scheduling uses an embedded static PowerShell script calling Task Scheduler COM. Paths and modes are passed as separate arguments. PowerShell runs during explicit installation, inspection and removal; the scheduled collector uses the hidden Rust helper.

The resource harness uses Python with Windows lifetime process counters. Sampled temporary disk usage is a lower bound. Production logs contain aggregate summary counts. Snapshot consistency is best-effort, and `schedule status` reports configuration rather than last-run health.

## Reproduction

Build with `cargo build --locked --release --bins --example synthetic`, then use `scripts/smoke.py` for the generated CLI journey. `scripts/measure.py --help` describes collector measurements. Store detailed local results under the ignored `.local-acceptance/` directory. Test sources, runtime validation and CI remain enabled.
