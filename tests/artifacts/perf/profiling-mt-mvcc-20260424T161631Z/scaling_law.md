# Scaling Law Readout

## Primary 500-row Matrix

| Threads | observed fsqlite wps | speedup vs 1T | efficiency vs ideal | note |
|--------:|---------------------:|--------------:|--------------------:|---|
| 1 | 265,987 | 1.000x | 100.0% | baseline |
| 2 | 9,559 | 0.036x | 1.8% | anti-scaling cliff |
| 4 | 7,940 | 0.030x | 0.7% | serialized/contended regime |
| 8 | 9,274 | 0.035x | 0.4% | high tail latency |
| 12 | 11,111 | 0.042x | 0.3% | high tail latency |

This is not an Amdahl-limited curve; it is negative scaling. Adding a second writer introduces a conflict/wait/retry regime that overwhelms the extra work capacity.

## 5000-row Amortization Probe

| Threads | observed fsqlite wps | speedup vs 1T | efficiency vs ideal | note |
|--------:|---------------------:|--------------:|--------------------:|---|
| 1 | 584,167 | 1.000x | 100.0% | setup amortized |
| 2 | 79,201 | 0.136x | 6.8% | cliff remains |
| 4 | 66,562 | 0.114x | 2.8% | serialized/contended regime |
| 8 | 68,950 | 0.118x | 1.5% | flat after cliff |

The amortized run raises absolute throughput but preserves the cliff. That separates two issues:

1. Short-sample measurements include connection/cache lifecycle overhead.
2. Even after amortization, concurrent writers converge to a low-throughput conflict regime.

## Tail Behavior

The 500-row matrix has p95/p99 fsqlite latencies above 5 seconds at 8 threads and above 10 seconds at 12 threads, despite zero failed rows in those runs. The 16-thread probe crosses from high tail latency into hard failure with snapshot conflicts on page 2.
