# Per-Core Parallel WAL Design Contract (`bd-3wop3.1.1`)

## Purpose

This document is the reference contract for Track D1.a:

- per-core append lanes and local staging,
- commit certificates,
- pager visibility publication,
- recovery/checkpoint semantics,
- safe-mode fallback and operator controls,
- optional decision-plane policy with explicit disable/fallback rules.

It supersedes the older `bd-ncivz.*` prototype notes. Future D1.b/D1.c/D1.d
and E3.2 work must implement this contract rather than reinterpret the parallel
WAL shape ad hoc.

## Deterministic Data Plane

### Unit of parallel staging

The parallel data plane stages one **lane batch** per writer-owned lane:

- a lane batch is the ordered set of `WalRecord`s emitted by one transaction on
  one lane during one sealing interval,
- `WalRecord` remains the lane-local mutation unit in
  `crates/fsqlite-wal/src/per_core_buffer.rs`,
- lane ownership is stable for the transaction lifetime; no transaction may
  append across multiple lanes.

### Commit ordering

Commit order is defined only by `CommitSeq` and is never inferred from wall
 clock, epoch number, lane number, or segment file order.

- lane-local staging is parallel and unordered relative to other lanes,
- the combiner assigns or confirms a contiguous `CommitSeq` interval,
- the combiner emits one `ParallelWalCommitCertificate` covering that interval,
- pager publication may only expose the certificate's `commit_seq_hi` after the
  certificate is durable.

### Exact irreducible ordered residue

The ordered residue is deliberately tiny and explicit:

1. assign or finalize the `CommitSeq` interval,
2. durably write the commit certificate,
3. publish pager visibility metadata derived from that certificate.

Everything before residue step 1 is lane-local. Recovery, checkpoint, and
readers must treat the certificate as the proof object that bridges lane-local
 staging to globally ordered visibility.

## Commit Certificate Record

The production proof object is `ParallelWalCommitCertificate` in
`crates/fsqlite-wal/src/parallel_wal.rs`.

### Required fields

| Field | Meaning |
| --- | --- |
| `format_version` | Stable certificate schema version |
| `residue` | Declared ordered-residue contract (`CommitCertificateThenPublish`) |
| `certificate_epoch` | Seal/drain epoch that produced the certificate |
| `commit_seq_lo` / `commit_seq_hi` | Closed commit interval covered by the certificate |
| `durable_segment_epoch` | Durable segment generation that contains the committed lane batches |
| `lane_count` | Number of lanes contributing evidence |
| `lane_record_counts` | Per-lane batch cardinality used for replay/audit |
| `db_size_pages` | Post-commit database size visible after publication |
| `page_set_size` | Published page-plane cardinality associated with the commit |
| `certificate_crc32c` | Certificate-level integrity checksum |
| `fallback_active` | Whether the conservative path owned this commit |

### Certificate semantics

- no commit is externally visible without a durable certificate,
- certificates must cover contiguous `CommitSeq` intervals with no gaps or
  overlaps,
- certificate durability is the handoff point between WAL append and pager
  publication,
- shadow-compare mode must produce a conservative certificate candidate and
  compare all contract fields before publication.

## Pager Visibility Publication

The pager handoff object is `ParallelWalPublicationIntent` in
`crates/fsqlite-pager/src/pager.rs`.

Publication rules:

- `visible_commit_seq` is the highest commit sequence made visible to readers,
- `page_plane_visible_commit_seq` may lag `visible_commit_seq` only when the
  page plane intentionally forces cache/inner fallback instead of serving stale
  published pages,
- `PagerPublishedSnapshot` is the post-publish image; it must correspond to a
  durable publication intent derived from exactly one certificate interval,
- publication must be monotone in `visible_commit_seq`.

## Recovery and Checkpoint Consumption

### Recovery contract

Recovery consumes lane batches only through certificate order:

- segment replay order is certificate interval order, not raw lane order,
- if recovery sees a lane batch without a matching durable certificate, that
  batch is ignored as uncommitted residue,
- if recovery sees a certificate gap, checksum mismatch, or interval overlap,
  it must force conservative recovery and stop trusting the parallel path.

### Checkpoint contract

Checkpoint uses the same visibility boundary:

- only pages at or below the highest durable published certificate may be
  checkpointed,
- checkpoint metadata must record the certificate epoch / `commit_seq_hi` it
  consumed,
- checkpoint overlap with a certificate interval still being published forces
  conservative/safe mode for that interval.

## Safe-Mode Fallback and Operator Control Surface

The explicit runtime knobs live in `ParallelWalControlSurface` and
`ParallelWalOperatingMode`.

### Modes

- `Auto`: parallel data plane enabled; optional controller may tune batching
  within caps
- `Conservative`: force serialized append/commit/publish for debugging,
  bisecting, and oracle comparison
- `ShadowCompare`: parallel data plane remains authoritative only while a
  conservative proof run agrees on certificate/publication facts

### Required overrides

- lane-count override: hard cap or exact lane count
- helper-lane budget override: cap on auxiliary flush/combine helpers
- max batch bytes override
- max flush delay override
- shadow-compare sampling override

### Deterministic fallback triggers

The fallback reason taxonomy is fixed by `ParallelWalFallbackReason`:

- `OperatorForced`
- `LaneOverflow`
- `CertificateGap`
- `CertificateChecksumMismatch`
- `PublicationMismatch`
- `RecoveryGap`
- `CheckpointConflict`
- `ControllerEvidenceLost`
- `ControllerCalibrationStale`

Any of these forces `Conservative` behavior for the affected interval. The
parallel path may only re-arm after an explicit clean-state transition verified
by the validation surface.

## Decision-Plane Contract

The optional controller is policy-as-data only. Correctness does not depend on
it.

Runtime schema:

- actions: `KeepCurrent`, `SealEpochNow`, `IncreaseLaneBudget`,
  `DecreaseLaneBudget`, `ForceConservative`
- record shape: `ParallelWalDecisionRecord`
- logging schema: `ParallelWalTraceRecord`

### Conservative baseline

If the controller is disabled or evidence goes bad, runtime behavior must be
equivalent to:

- `ParallelWalOperatingMode::Conservative`, or
- `Auto` with no tuned overrides and immediate fallback to conservative on the
  first bad-evidence event.

### State/action table

| State slice | Allowed actions | Loss focus |
| --- | --- | --- |
| low occupancy, no fallback pressure | `KeepCurrent`, `DecreaseLaneBudget` | avoid helper waste |
| high occupancy, low lag | `KeepCurrent`, `IncreaseLaneBudget` | reduce seal latency |
| high lag, certificate backlog | `SealEpochNow`, `IncreaseLaneBudget` | bound publish delay |
| publication mismatch, recovery anomaly, or low confidence | `ForceConservative` | correctness over throughput |

### Decision record requirements

Every adaptive action must emit:

- `policy_id`
- `policy_version`
- `decision_id`
- chosen action
- confidence / posterior (`confidence_bps`)
- expected loss
- top evidence terms
- counterfactual action
- counterfactual regret delta
- fallback-active bit

### Policy artifact identity and calibration telemetry

The controller is policy-as-data. The data-plane must be able to prove which
policy artifact produced each decision and whether calibration remained valid.

Required policy identity fields:

- `policy_id`
- `policy_version`
- `policy_artifact_hash`

Required calibration fields:

- `calibration_window_ms`
- `calibration_confidence_bps`
- `calibration_last_ok_ts`

If calibration confidence drops below the contract threshold or the calibration
window expires without a validated refresh, the controller must emit
`ControllerCalibrationStale` and force conservative mode.

## Timescale Separation

The D1 controller is the fastest control loop, but it still only tunes batch and
lane behavior. It must not subsume other controllers.

- D1 controller: flush timing, lane budget, helper-lane use; sub-second to
  second scale
- E4 admission control: writer admission / pressure caps; slower than D1 and
  only constrains available work
- E6 placement policy: lane/core placement and topology mapping; slower than E4
  and only changes between transactions
- later cache/routing controllers: may adapt read/write routing, but must treat
  certificate/publication semantics as fixed invariants

## Invariants Ledger

`INV-D1-1` Commit order is defined solely by `CommitSeq`.

`INV-D1-2` No publication without a durable certificate.

`INV-D1-3` Certificate intervals are contiguous, non-overlapping, and auditable.

`INV-D1-4` Pager publication is monotone in `visible_commit_seq`.

`INV-D1-5` Recovery replays only certificate-backed intervals.

`INV-D1-6` Checkpoint never crosses an unpublished certificate boundary.

`INV-D1-7` Conservative mode is semantically equivalent to the parallel path for
the same committed interval.

`INV-D1-8` Disabling the decision plane never changes correctness, only tuning.

## Logging Contract Schema

Every lane, combiner, checkpoint, recovery, and controller event must expose
the fields represented by `ParallelWalTraceRecord`, plus scenario-level
envelope fields supplied by the caller:

- `trace_id`
- `decision_id` when applicable
- `component`
- `mode`
- `lane_id` when applicable
- `epoch`
- `commit_seq_lo`
- `commit_seq_hi`
- `checkpoint_epoch` when applicable
- `recovery_epoch` when applicable
- `fallback_active`
- `fallback_reason`
- `policy_id`
- `policy_version`
- `policy_artifact_hash`
- `calibration_window_ms`
- `calibration_confidence_bps`
- `calibration_last_ok_ts`
- outer envelope: `run_id`, `scenario_id`, `bead_id`, `artifact_path`

## Validation Surface

The named validation entrypoint is:

- `scripts/verify_d1_parallel_wal_design_contract.sh`

Minimum scenarios and expected outcomes:

1. commit-order table test
   Expected: no interval gaps/overlaps; certificate fields agree with replay order
2. lane reordering replay
   Expected: recovery replays by certificate order, not raw lane arrival order
3. checkpoint overlap
   Expected: checkpoint stops at the last published certificate boundary
4. forced conservative mode
   Expected: certificate/publication semantics remain equivalent
5. shadow-compare divergence
   Expected: `PublicationMismatch` or related fallback reason forces conservative mode
6. controller evidence loss
   Expected: `ControllerEvidenceLost` forces conservative mode and logs a decision record

## Immediate Implementation Bindings

- D1.b must implement lane batches and sealing without expanding the ordered residue
- D1.c must implement certificate durability and pager handoff exactly through
  the named schemas
- D1.d must prove recovery/checkpoint against this certificate boundary
- E3.2 must treat pager publication as certificate-derived metadata, not an
  independently interpreted plane
