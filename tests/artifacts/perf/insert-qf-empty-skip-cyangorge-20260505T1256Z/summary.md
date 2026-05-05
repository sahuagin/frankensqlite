# Direct INSERT QF empty-map skip candidate

Candidate:

- In `crates/fsqlite-core/src/connection.rs`, add an early return to
  `qf_record_insert` and `qf_record_delete` when `quotient_filters` is empty.
- Intended target: avoid per-row mutable QF maintenance lookup during
  direct-simple INSERT workloads that have not seeded a quotient filter.

Correctness gate:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo test -p fsqlite-core quotient_filter -- --nocapture
```

Result:

- Failed before benchmarking.
- `test_quotient_filter_short_circuits_absent_rowids_on_delete` failed with
  `expected >= 90 QF short-circuits, got 0`.
- `test_quotient_filter_delete_then_redelete_short_circuits` failed because the
  second delete of a removed rowid did not short-circuit through the QF.

Verdict:

Rejected and reverted. Empty-map QF maintenance is not a semantics-neutral
fast path because it interferes with the lazy seed/maintenance lifecycle used
by DELETE and UPDATE short-circuiting.
