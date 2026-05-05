# UPDATE/DELETE Focused Profile - CyanGorge - 2026-05-05 15:50 UTC

Focused profile after the full quick retarget matrix showed `write_single`
UPDATE/DELETE rows remain among the largest FrankenSQLite-over-C-SQLite gaps.
Source commit: `47869cc3f6837ffcb56a9ec63c6b6f2e7d1d8bb6`.

Benchmark command:

```bash
FSQLITE_BENCH_PROFILE_DML=1 \
  /data/tmp/frankensqlite-cyangorge-walchk-target/release-perf/comprehensive-bench \
  --quick \
  --filter update \
  --json-out tests/artifacts/perf/update-delete-profile-cyangorge-20260505T1555Z/report.json \
  --no-html
```

CPU sample command:

```bash
perf record -F 999 -g \
  -o tests/artifacts/perf/update-delete-profile-cyangorge-20260505T1555Z/perf.data \
  -- /data/tmp/frankensqlite-cyangorge-dml-profile-target/release-perf/perf-update-delete 10000 250 both
```

Perf note: kernel symbols were restricted by host settings, but user-space
symbols resolved.

## Matrix Result

| Scenario | C SQLite median | FrankenSQLite median | Ratio |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 0.087453 ms | 0.336911 ms | 3.8525x |
| 100 rows / delete 5 rows | 0.085280 ms | 0.334026 ms | 3.9168x |
| 1000 rows / update 100 rows | 0.443600 ms | 1.243659 ms | 2.8036x |
| 1000 rows / delete 50 rows | 0.439664 ms | 1.180150 ms | 2.6842x |
| 10000 rows / update 1000 rows | 3.945168 ms | 9.211018 ms | 2.3348x |
| 10000 rows / delete 500 rows | 3.632943 ms | 9.227008 ms | 2.5398x |

Section aggregate:

- Average ratio: `3.0219x`
- Geomean ratio: `2.9606x`
- Median ratio: `2.8036x`
- p90/p99 ratio: `3.9168x`

## Built-In DML Counters

Representative 100-row rows:

- UPDATE: `setup_us=195.0`, `begin_us=8.3`, `prepare_us=28.6`,
  `mutate_us=15.0`, `commit_us=18.3`, `direct_update=10`,
  `vdbe_opcodes=0`.
- DELETE: `setup_us=141.5`, `begin_us=6.4`, `prepare_us=22.7`,
  `mutate_us=16.8`, `commit_us=22.4`, `direct_delete=5`,
  `vdbe_opcodes=0`.

Representative 10K-row rows:

- UPDATE: `setup_us=7055.1`, `mutate_us=1177.0`, `commit_us=623.0`,
  `commit_roundtrip_ns=609511`, `direct_update=1000`, `vdbe_opcodes=0`.
- DELETE: `setup_us=8232.7`, `mutate_us=1006.7`, `commit_us=949.4`,
  `commit_roundtrip_ns=925523`, `direct_delete=500`, `vdbe_opcodes=0`.

Interpretation:

- The benchmark row includes setup/prepopulation work inside the measured
  closure for both engines.
- The FrankenSQLite path is already on direct UPDATE/DELETE, not VDBE dispatch.
- Large rows are dominated by setup/prepopulation plus commit publication.
- Small rows are dominated by fixed create/BEGIN/prepare/mutate/COMMIT costs.

## CPU Sample

The narrow `perf-update-delete 10000 250 both` run reported:

- Total: `2582 ms`
- Populate: `1712 ms`
- Update: `451 ms`
- Delete: `277 ms`
- Per-row update: `1806 ns`
- Per-row delete: `2219 ns`

Top children from `perf-report-children.txt`:

- `execute_prepared_with_params_after_background_status`: `81.14%`
- `execute_precompiled_prepared_insert_fast`: `59.44%`
- `execute_prepared_direct_simple_insert`: `56.37%`
- `table_try_append_cached_rightmost_leaf_hint`: `24.74%`
- `try_append_table_leaf_payload_in_place_no_overflow`: `20.06%`
- `SharedTxnPageIo::write_page_data`: `11.65%`
- `eval_prepared_direct_simple_insert_expr`: `5.93%`
- `eval_prepared_direct_simple_insert_concat_chain`: `4.36%`
- `serialize_record_iter_with_precomputed_header_into`: `2.80%`
- `push_prepared_direct_simple_insert_value`: `2.43%`

Top self-time entries from `perf-report-nochildren.txt`:

- `__memmove_avx_unaligned_erms`: `10.64%`
- `execute_prepared_direct_simple_insert`: `9.12%`
- `_int_malloc`: `3.69%`
- `eval_prepared_direct_simple_insert_expr`: `2.98%`
- `push_prepared_direct_simple_insert_value`: `2.69%`
- `serialize_record_iter_with_precomputed_header_into`: `2.47%`
- `SharedTxnPageIo::write_page_internal`: `2.43%`
- `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface`: `2.42%`

## No-Retry Fences Checked

The current profile intersects several known rejected ideas:

- Do not retry direct UPDATE fixed-width REAL header/payload patches.
- Do not retry direct single-rowid DELETE lowering.
- Do not retry direct DML cursor scratch routing.
- Do not retry simple schema-proof microbatch carry for direct UPDATE/DELETE.
- Do not retry session-shared page-1 synthetic hint state just because the
  `clear_stale_synthetic_pending_commit_surface` stack is visible.
- Do not retry direct INSERT concat/text pooling, integer placeholder text
  caching, or expression-shape special cases unless a fresh insert-section A/B
  wins the matrix.
- Do not retry retained-leaf writer callbacks, simple rightmost-page handoffs,
  or private `:memory:` WAL bypass as standalone insert shortcuts.

## Retargeting Conclusion

This UPDATE/DELETE slice is not a VDBE UPDATE/DELETE problem. It is mostly the
same direct INSERT setup and page-write path that dominates the write_bulk
matrix, with a smaller true DML tail. The next viable source candidate should
come from either:

1. a new insert-path lever that is materially different from the recorded
   text-pooling / expression-specialization / rightmost-cache / WAL-bypass
   rejects, or
2. an isolated pure-mutation profiler that removes setup from the sample before
   attempting another direct UPDATE/DELETE mutation change.

The alien-graveyard scan did not produce a safe drop-in primitive for this
profile. B-epsilon trees, latch-free MVCC redesigns, and parallel WAL are
architecture-scale levers; none should be started from this microprofile without
a broader design bead and correctness proof plan.
