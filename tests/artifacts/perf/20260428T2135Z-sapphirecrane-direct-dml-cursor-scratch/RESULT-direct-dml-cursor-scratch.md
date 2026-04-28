# Direct DML cursor scratch reuse rejection

Run: 2026-04-28T21:35Z

Scenario: `perf-update-delete 10000 100 both`

Goal: test whether direct UPDATE/DELETE should route payload scratch through
`prepared_direct_insert_record_scratch` so `prepared_direct_insert_cell_scratch`
can be swapped into the fresh `BtCursor` as reusable cell-assembly storage.

## Evidence

Initial separate runs were noisy and misleading:

| Binary | Mean | Stddev | Runs |
|---|---:|---:|---:|
| clean parent `3e0382bc` | 1.459 s | 0.107 s | 12 |
| scratch-routing patch | 1.297 s | 0.071 s | 12 |

The interleaved comparison rejected the optimization:

| Binary | Mean | Stddev | Runs |
|---|---:|---:|---:|
| clean parent `3e0382bc` | 1.262 s | 0.019 s | 20 |
| scratch-routing patch | 1.270 s | 0.021 s | 20 |

Hyperfine summary: clean parent ran `1.01 +/- 0.02` times faster.

## Conclusion

Reject and revert. The extra `RefCell` borrows and cursor scratch swaps cost
slightly more than any cell-buffer reuse they enable in this mixed
insert/update/delete workload.

Keep the existing direct UPDATE/DELETE scratch routing for now. A future
attempt needs a broader cursor-owned mutation scratch API that amortizes
`defrag_ptrs_scratch` / `defrag_cells_scratch` as well, and should be judged
against an update/delete-isolated benchmark rather than the populate-heavy
`perf-update-delete` default.

## Artifacts

- `hyperfine-baseline-both.json`
- `hyperfine-patch-both.json`
- `hyperfine-compare-both.json`
- `smoke-baseline.txt`
- `smoke-patch.txt`
