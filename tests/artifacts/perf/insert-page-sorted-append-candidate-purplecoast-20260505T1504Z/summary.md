# Insert Page-Sorted Append Candidate

- Candidate: in `crates/fsqlite-pager/src/pager.rs`, add a fast path to
  `insert_page_sorted` for the common monotonic append/equal cases before
  falling back to `binary_search_by_key`.
- Worktree: isolated clean worktree at commit `f55060ff`; the shared dirty
  `pager.rs` file was not edited or staged by this run.
- Baseline:
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`.
- Candidate:
  `tests/artifacts/perf/insert-page-sorted-append-candidate-purplecoast-20260505T1504Z/report.json`.

## Result

Rejected. The append fast path improved a few individual rows, but the primary
insert score and write-single section moved the wrong way.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Avg ratio | `2.4610x` | `2.4231x` |
| Geomean ratio | `2.3623x` | `2.3470x` |
| Weighted score | `1.6991` | `1.7171` |
| write_bulk geomean | `2.5153x` | `2.4909x` |
| write_single geomean | `1.4908x` | `1.5168x` |

Selected absolute medians:

| Row | Baseline F median | Candidate F median |
| --- | ---: | ---: |
| single transaction `tiny_1col` 100 | `0.267 ms` | `0.274 ms` |
| single transaction `small_3col` 100 | `0.293 ms` | `0.208 ms` |
| single transaction `large_10col` 10K | `36.165 ms` | `36.060 ms` |
| record-size `large_10col` 10K | `37.056 ms` | `36.568 ms` |

## Disposition

Do not retry the simple last-page append/equal branch in `insert_page_sorted`
as a standalone insert optimization. The measured benefit is too small and
uneven, and the insert weighted score regressed.
