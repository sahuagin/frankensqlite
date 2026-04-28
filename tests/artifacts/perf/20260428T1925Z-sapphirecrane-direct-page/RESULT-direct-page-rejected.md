# Direct Leaf Payload Writer — Rejected

Date: 2026-04-28

## Hypothesis

`perf-update-delete 10000 100 both` still shows meaningful time in record
serialization and payload copying on the prepared direct INSERT populate path.
`fsqlite-btree` already had an unused writer-callback primitive that can carve
the leaf-cell payload slice first and let the caller serialize a SQLite record
directly into the page.

The tested patch wired that primitive for retained rightmost-leaf direct INSERT
appends when `PreparedDirectSimpleInsert::cached_record_header` could prove the
exact payload size. If the hinted leaf was stale/full or needed overflow, the
old `record_scratch -> table_insert` path remained the fallback.

## Result

Baseline current-head artifact:
`tests/artifacts/perf/20260428T1847Z-sapphirecrane-current-head/hyperfine-current.json`

Candidate artifact:
`tests/artifacts/perf/20260428T1925Z-sapphirecrane-direct-page/hyperfine-direct-page.json`

Same command for both:

```bash
perf-update-delete 10000 100 both
```

| Build | Mean | Median | Min | Max |
|---|---:|---:|---:|---:|
| Current HEAD | 1.261932s | 1.271083s | 1.215167s | 1.327614s |
| Direct page writer patch | 1.290519s | 1.284636s | 1.235854s | 1.357537s |

Mean regressed by 2.27%. Median regressed by 1.07%.

## Decision

Rejected and rolled back. The closure-based try path and extra exact-size
measurement cost more than the avoided `record_scratch` payload copy in this
scenario. The candidate diff is preserved in `direct-page.diff` as a
negative-result artifact.

