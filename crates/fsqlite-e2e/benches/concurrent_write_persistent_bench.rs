//! Benchmark: Real persistent concurrent-writer throughput.
//!
//! Bead: bd-l9k8e.8 (C8)
//!
//! THIS IS THE ONLY BENCHMARK THAT MATTERS.
//!
//! FrankenSQLite's thesis: page-level MVCC enables concurrent writers where
//! SQLite serializes them.  This benchmark measures:
//!
//! - N writer threads (2, 4, 8, 16)
//! - Each writer INSERTs into a DIFFERENT table (guaranteeing different pages)
//! - File-backed database with WAL mode
//! - Prepared statements on both sides
//!
//! Success criterion: FrankenSQLite shows >1.5x throughput over SQLite at N>=4
//! writers for non-conflicting workloads.  Theoretical improvement is Nx.
//!
//! Metrics captured:
//! - Wall-clock throughput (ops/sec) at each thread count
//! - Per-thread commit latency histogram (p50, p95, p99, max)
//! - Conflict/retry count (SQLITE_BUSY retries for C SQLite)
//!
//! Results (2026-03-20):
//! - 2 threads: FrankenSQLite 8.97 Kelem/s vs C SQLite 2.32 Kelem/s (**3.87x faster**)
//! - 4 threads: FrankenSQLite 8.60 Kelem/s vs C SQLite 2.32 Kelem/s (**3.71x faster**)
//! - 8 threads: FrankenSQLite 1.58 Kelem/s vs C SQLite 2.36 Kelem/s (0.67x - degraded)
//! - 16 threads: FrankenSQLite 1.29 Kelem/s vs C SQLite 2.42 Kelem/s (0.53x - degraded)
//!
//! The thesis is validated at 2-4 threads. At 8+ threads, internal contention
//! causes throughput degradation below C SQLite. Investigation needed.
//!
//! Fixed issues:
//! - 16-thread corruption (page 0x00 type flag) - fixed via MVCC snapshot db_size guard
//!
//! Remaining issues:
//! - Performance degrades at 8+ threads (internal lock contention suspected)
//! - p50 latency increases dramatically at higher thread counts
//!
//! Optional machine-readable capture:
//! - Set `FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR=/path/to/dir`
//! - The benchmark writes `provenance.json` once and appends per-iteration
//!   records to `samples.jsonl` without changing default stderr output

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use fsqlite::{FrankenError, SqliteValue};
use fsqlite_wal::ConsolidationMetricsSnapshot;
use serde::Serialize;

const ROWS_PER_THREAD: i64 = 1000;
/// Maximum retries before giving up on a transaction (applies to both engines).
const MAX_TXN_RETRIES: u32 = 100;
const PERSISTENT_PHASE_CAPTURE_DIR_ENV: &str = "FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR";
const PERSISTENT_PHASE_CAPTURE_PROVENANCE_SCHEMA_V1: &str =
    "fsqlite-e2e.persistent_phase_capture_provenance.v1";
const PERSISTENT_PHASE_CAPTURE_SAMPLE_SCHEMA_V1: &str =
    "fsqlite-e2e.persistent_phase_capture_sample.v1";

// ─── PRAGMA helpers ─────────────────────────────────────────────────────

fn run_fsqlite_pragma(conn: &fsqlite::Connection, pragma: &str) {
    conn.execute(pragma)
        .unwrap_or_else(|error| panic!("failed to execute benchmark pragma `{pragma}`: {error:?}"));
}

fn apply_setup_pragmas_fsqlite(conn: &fsqlite::Connection) {
    for pragma in [
        "PRAGMA page_size = 4096;",
        "PRAGMA journal_mode = WAL;",
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA cache_size = -64000;",
        "PRAGMA fsqlite.concurrent_mode = ON;",
    ] {
        run_fsqlite_pragma(conn, pragma);
    }
}

fn apply_session_pragmas_fsqlite(conn: &fsqlite::Connection) {
    for pragma in [
        "PRAGMA journal_mode = WAL;",
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA cache_size = -64000;",
        "PRAGMA fsqlite.concurrent_mode = ON;",
    ] {
        run_fsqlite_pragma(conn, pragma);
    }
}

fn is_retryable_fsqlite_error(error: &FrankenError) -> bool {
    matches!(
        error,
        FrankenError::Busy | FrankenError::BusyRecovery | FrankenError::BusySnapshot { .. }
    )
}

fn is_duplicate_insert_after_retry(error: &FrankenError) -> bool {
    // Check for proper constraint errors
    if matches!(
        error,
        FrankenError::PrimaryKeyViolation | FrankenError::UniqueViolation { .. }
    ) {
        return true;
    }
    // Also check for VDBE constraint errors (code 19) wrapped as Internal
    if let FrankenError::Internal(msg) = error {
        if msg.contains("code 19:") && msg.contains("PRIMARY KEY") {
            return true;
        }
        if msg.contains("code 19:") && msg.contains("UNIQUE") {
            return true;
        }
    }
    false
}

fn is_corruption_error(error: &FrankenError) -> bool {
    matches!(
        error,
        FrankenError::DatabaseCorrupt { .. } | FrankenError::WalCorrupt { .. }
    )
}

fn create_table_sql(table_id: usize) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS bench_{table_id} (id INTEGER PRIMARY KEY, name TEXT, score INTEGER);"
    )
}

fn insert_sql(table_id: usize) -> String {
    format!("INSERT INTO bench_{table_id} VALUES (?1, ('t' || ?1), (?1 * 7));")
}

fn criterion_config() -> Criterion {
    Criterion::default().configure_from_args()
}

#[derive(Debug, Clone, Serialize)]
struct PersistentPhaseCaptureProvenance {
    schema_version: &'static str,
    benchmark: &'static str,
    output_dir_env: &'static str,
    rows_per_thread: i64,
    max_txn_retries: u32,
    current_dir: String,
    current_exe: Option<String>,
    argv: Vec<String>,
    hostname: Option<String>,
    kernel_release: Option<String>,
    criterion_emission_scope: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct PersistentLatencySummary {
    sample_count: u64,
    p50_us: u64,
    p95_us: u64,
    p99_us: u64,
    max_us: u64,
}

#[derive(Debug, Clone, Serialize)]
struct PersistentPhaseCaptureSample {
    schema_version: &'static str,
    timestamp_unix_ms: u64,
    benchmark_group: String,
    engine: &'static str,
    counter_name: &'static str,
    counter_value: u64,
    concurrency: usize,
    rows_per_thread: i64,
    total_rows: u64,
    latency_us: PersistentLatencySummary,
    phase_metrics: Option<ConsolidationMetricsSnapshot>,
    phase_timing_report: Option<String>,
    flusher_lock_wait_fraction_basis_points: Option<u64>,
    lock_topology_limited: Option<bool>,
}

fn persistent_phase_capture_dir() -> Option<PathBuf> {
    std::env::var_os(PERSISTENT_PHASE_CAPTURE_DIR_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn read_trimmed_file(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|contents| contents.trim().to_owned())
        .filter(|contents| !contents.is_empty())
}

fn persistent_phase_capture_provenance() -> PersistentPhaseCaptureProvenance {
    PersistentPhaseCaptureProvenance {
        schema_version: PERSISTENT_PHASE_CAPTURE_PROVENANCE_SCHEMA_V1,
        benchmark: "concurrent_write_persistent_bench",
        output_dir_env: PERSISTENT_PHASE_CAPTURE_DIR_ENV,
        rows_per_thread: ROWS_PER_THREAD,
        max_txn_retries: MAX_TXN_RETRIES,
        current_dir: std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_owned()),
        current_exe: std::env::current_exe()
            .ok()
            .map(|path| path.display().to_string()),
        argv: std::env::args_os()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        hostname: std::env::var("HOSTNAME")
            .ok()
            .filter(|hostname| !hostname.is_empty())
            .or_else(|| read_trimmed_file("/etc/hostname")),
        kernel_release: read_trimmed_file("/proc/sys/kernel/osrelease"),
        criterion_emission_scope: "every completed Criterion batched iteration appends one record; warmup and measurement phases are not distinguished by this harness",
    }
}

fn ensure_persistent_phase_capture_provenance(output_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(output_dir)?;
    let provenance_path = output_dir.join("provenance.json");
    if provenance_path.exists() {
        return Ok(());
    }
    let payload = serde_json::to_string_pretty(&persistent_phase_capture_provenance())
        .map_err(std::io::Error::other)?;
    fs::write(provenance_path, payload.as_bytes())
}

fn duration_micros_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn persistent_latency_summary(sorted: &[Duration]) -> PersistentLatencySummary {
    PersistentLatencySummary {
        sample_count: u64::try_from(sorted.len()).unwrap_or(u64::MAX),
        p50_us: duration_micros_u64(percentile(sorted, 50.0)),
        p95_us: duration_micros_u64(percentile(sorted, 95.0)),
        p99_us: duration_micros_u64(percentile(sorted, 99.0)),
        max_us: duration_micros_u64(sorted.last().copied().unwrap_or(Duration::ZERO)),
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn flusher_lock_wait_fraction_basis_points(metrics: &ConsolidationMetricsSnapshot) -> Option<u64> {
    let lock_wait_total = metrics.flusher_lock_wait_us_total();
    let wal_service_total = metrics.wal_service_us_total();
    let total = lock_wait_total.saturating_add(wal_service_total);
    (total > 0).then_some(lock_wait_total.saturating_mul(10_000) / total)
}

fn maybe_write_persistent_phase_capture(sample: &PersistentPhaseCaptureSample) {
    let Some(output_dir) = persistent_phase_capture_dir() else {
        return;
    };
    if let Err(error) = ensure_persistent_phase_capture_provenance(&output_dir) {
        eprintln!(
            "[persistent phase capture] failed to write provenance in {}: {error}",
            output_dir.display()
        );
        return;
    }
    let sample_path = output_dir.join("samples.jsonl");
    let encoded = match serde_json::to_string(sample) {
        Ok(encoded) => encoded,
        Err(error) => {
            eprintln!("[persistent phase capture] failed to serialize sample: {error}");
            return;
        }
    };
    let write_result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&sample_path)?;
        writeln!(file, "{encoded}")?;
        Ok(())
    })();
    if let Err(error) = write_result {
        eprintln!(
            "[persistent phase capture] failed to append {}: {error}",
            sample_path.display()
        );
    }
}

/// Compute percentiles from a sorted slice of latencies.
fn percentile(sorted: &[Duration], pct: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

// ─── C SQLite concurrent writers (file-backed WAL) ──────────────────────

fn bench_concurrent_csqlite_persistent(c: &mut Criterion, n_threads: usize, label: &str) {
    #[allow(clippy::cast_possible_wrap)]
    let total_rows = n_threads as u64 * ROWS_PER_THREAD as u64;
    let mut group = c.benchmark_group(label);
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(45));
    group.throughput(Throughput::Elements(total_rows));

    group.bench_function("csqlite_concurrent_persistent", |b| {
        b.iter_batched(
            || {
                let tmp = tempfile::NamedTempFile::new().unwrap();
                let path = tmp.path().to_str().unwrap().to_owned();
                {
                    let setup = rusqlite::Connection::open(&path).unwrap();
                    setup
                        .execute_batch(
                            "PRAGMA page_size = 4096;\
                             PRAGMA journal_mode = WAL;\
                             PRAGMA synchronous = NORMAL;\
                             PRAGMA cache_size = -64000;",
                        )
                        .unwrap();
                    // Create separate tables for each thread
                    for tid in 0..n_threads {
                        setup.execute_batch(&create_table_sql(tid)).unwrap();
                    }
                }
                let retry_count = Arc::new(AtomicU64::new(0));
                (tmp, path, retry_count)
            },
            |(_tmp, path, retry_count)| {
                let barrier = Arc::new(Barrier::new(n_threads));
                let latencies: Arc<Vec<std::sync::Mutex<Vec<Duration>>>> = Arc::new(
                    (0..n_threads)
                        .map(|_| std::sync::Mutex::new(Vec::with_capacity(ROWS_PER_THREAD as usize)))
                        .collect(),
                );

                let handles: Vec<_> = (0..n_threads)
                    .map(|tid| {
                        let p = path.clone();
                        let bar = barrier.clone();
                        let retries = retry_count.clone();
                        let lat = latencies.clone();
                        thread::spawn(move || {
                            let conn = rusqlite::Connection::open(&p).unwrap();
                            conn.execute_batch(
                                "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;",
                            )
                            .unwrap();
                            let insert_stmt = insert_sql(tid);
                            let mut stmt = conn.prepare(&insert_stmt).unwrap();
                            bar.wait();

                            // Each row is its own transaction for realistic commit latency
                            for i in 0..ROWS_PER_THREAD {
                                let start = Instant::now();
                                let mut begin_retries = 0u32;
                                loop {
                                    match conn.execute_batch("BEGIN IMMEDIATE") {
                                        Ok(()) => break,
                                        Err(e) => {
                                            let msg = e.to_string();
                                            if msg.contains("BUSY") || msg.contains("locked") {
                                                retries.fetch_add(1, Ordering::Relaxed);
                                                begin_retries += 1;
                                                if begin_retries >= MAX_TXN_RETRIES {
                                                    panic!("BEGIN failed after {MAX_TXN_RETRIES} retries: {e}");
                                                }
                                                std::thread::sleep(Duration::from_micros(100));
                                            } else {
                                                panic!("BEGIN failed: {e}");
                                            }
                                        }
                                    }
                                }
                                stmt.execute(rusqlite::params![i]).unwrap();
                                let mut commit_retries = 0u32;
                                loop {
                                    match conn.execute_batch("COMMIT") {
                                        Ok(()) => break,
                                        Err(e) => {
                                            let msg = e.to_string();
                                            if msg.contains("BUSY") || msg.contains("locked") {
                                                retries.fetch_add(1, Ordering::Relaxed);
                                                commit_retries += 1;
                                                if commit_retries >= MAX_TXN_RETRIES {
                                                    panic!("COMMIT failed after {MAX_TXN_RETRIES} retries: {e}");
                                                }
                                                std::thread::sleep(Duration::from_micros(100));
                                            } else {
                                                panic!("COMMIT failed: {e}");
                                            }
                                        }
                                    }
                                }
                                let elapsed = start.elapsed();
                                lat[tid].lock().unwrap().push(elapsed);
                            }
                        })
                    })
                    .collect();

                for h in handles {
                    h.join().unwrap();
                }

                // Report metrics
                let total_retries = retry_count.load(Ordering::Relaxed);
                let mut all_latencies: Vec<Duration> = latencies
                    .iter()
                    .flat_map(|m| m.lock().unwrap().clone())
                    .collect();
                all_latencies.sort();

                let p50 = percentile(&all_latencies, 50.0);
                let p95 = percentile(&all_latencies, 95.0);
                let p99 = percentile(&all_latencies, 99.0);
                let max = all_latencies.last().copied().unwrap_or(Duration::ZERO);

                eprintln!(
                    "[C SQLite {n_threads}t] retries={total_retries}, p50={:?}, p95={:?}, p99={:?}, max={:?}",
                    p50, p95, p99, max
                );
                maybe_write_persistent_phase_capture(&PersistentPhaseCaptureSample {
                    schema_version: PERSISTENT_PHASE_CAPTURE_SAMPLE_SCHEMA_V1,
                    timestamp_unix_ms: unix_timestamp_ms(),
                    benchmark_group: format!("{label}/csqlite_concurrent_persistent"),
                    engine: "sqlite3",
                    counter_name: "retries",
                    counter_value: total_retries,
                    concurrency: n_threads,
                    rows_per_thread: ROWS_PER_THREAD,
                    total_rows,
                    latency_us: persistent_latency_summary(&all_latencies),
                    phase_metrics: None,
                    phase_timing_report: None,
                    flusher_lock_wait_fraction_basis_points: None,
                    lock_topology_limited: None,
                });
            },
            criterion::BatchSize::LargeInput,
        );
    });

    // FrankenSQLite with real concurrent writers
    group.bench_function("frankensqlite_concurrent_persistent", |b| {
        b.iter_batched(
            || {
                let tmp = tempfile::NamedTempFile::new().unwrap();
                let path = tmp.path().to_str().unwrap().to_owned();
                {
                    // Setup: create tables using a single connection
                    let setup = fsqlite::Connection::open(&path).unwrap();
                    apply_setup_pragmas_fsqlite(&setup);
                    for tid in 0..n_threads {
                        setup.execute(&create_table_sql(tid)).unwrap();
                    }
                }
                let conflict_count = Arc::new(AtomicU64::new(0));
                (tmp, path, conflict_count)
            },
            |(_tmp, path, conflict_count)| {
                let barrier = Arc::new(Barrier::new(n_threads));
                let latencies: Arc<Vec<std::sync::Mutex<Vec<Duration>>>> = Arc::new(
                    (0..n_threads)
                        .map(|_| std::sync::Mutex::new(Vec::with_capacity(ROWS_PER_THREAD as usize)))
                        .collect(),
                );

                let handles: Vec<_> = (0..n_threads)
                    .map(|tid| {
                        let p = path.clone();
                        let bar = barrier.clone();
                        let conflicts = conflict_count.clone();
                        let lat = latencies.clone();
                        thread::spawn(move || {
                            let conn = fsqlite::Connection::open(&p).unwrap();
                            apply_session_pragmas_fsqlite(&conn);
                            let insert_stmt = insert_sql(tid);
                            let stmt = conn.prepare(&insert_stmt).unwrap();
                            bar.wait();

                            for i in 0..ROWS_PER_THREAD {
                                // Each thread writes to its own table, so row IDs can match
                                // the SQLite side exactly without cross-thread collisions.
                                let row_id = i;
                                let start = Instant::now();
                                let mut retry_count = 0u32;

                                'txn: loop {
                                    // BEGIN CONCURRENT with retry
                                    loop {
                                        match conn.execute("BEGIN CONCURRENT") {
                                            Ok(_) => break,
                                            Err(e) => {
                                                if is_retryable_fsqlite_error(&e) {
                                                    conflicts.fetch_add(1, Ordering::Relaxed);
                                                    retry_count += 1;
                                                    if retry_count >= MAX_TXN_RETRIES {
                                                        panic!(
                                                            "BEGIN CONCURRENT failed after {MAX_TXN_RETRIES} retries: {e:?}"
                                                        );
                                                    }
                                                    std::thread::sleep(Duration::from_micros(
                                                        100 * u64::from(retry_count),
                                                    ));
                                                } else {
                                                    panic!("BEGIN CONCURRENT failed: {e:?}");
                                                }
                                            }
                                        }
                                    }

                                    // INSERT
                                    if let Err(e) = stmt.execute_with_params(&[SqliteValue::Integer(row_id)]) {
                                        if is_duplicate_insert_after_retry(&e) {
                                            // Row already exists (from previous retry that actually committed)
                                            let _ = conn.execute("ROLLBACK");
                                            break 'txn;
                                        }
                                        if is_retryable_fsqlite_error(&e)
                                            || matches!(e, FrankenError::SerializationFailure { .. })
                                        {
                                            // Snapshot conflict — rollback and retry
                                            conflicts.fetch_add(1, Ordering::Relaxed);
                                            let _ = conn.execute("ROLLBACK");
                                            retry_count += 1;
                                            if retry_count >= MAX_TXN_RETRIES {
                                                panic!("INSERT failed after {MAX_TXN_RETRIES} retries: {e:?}");
                                            }
                                            std::thread::sleep(Duration::from_micros(100 * u64::from(retry_count)));
                                            continue 'txn;
                                        }
                                        if is_corruption_error(&e) {
                                            let _ = conn.execute("ROLLBACK");
                                            panic!("CORRUPTION DETECTED: {e:?}");
                                        }
                                        panic!("INSERT failed: {e:?}");
                                    }

                                    // COMMIT with retry
                                    match conn.execute("COMMIT") {
                                        Ok(_) => break 'txn,
                                        Err(e) => {
                                            if is_retryable_fsqlite_error(&e)
                                                || matches!(e, FrankenError::SerializationFailure { .. })
                                            {
                                                conflicts.fetch_add(1, Ordering::Relaxed);
                                                let _ = conn.execute("ROLLBACK");
                                                retry_count += 1;
                                                if retry_count >= MAX_TXN_RETRIES {
                                                    panic!("COMMIT failed after {MAX_TXN_RETRIES} retries: {e:?}");
                                                }
                                                std::thread::sleep(Duration::from_micros(100 * u64::from(retry_count)));
                                                // Loop back to BEGIN CONCURRENT
                                            } else {
                                                panic!("COMMIT failed: {e:?}");
                                            }
                                        }
                                    }
                                }

                                let elapsed = start.elapsed();
                                lat[tid].lock().unwrap().push(elapsed);
                            }
                        })
                    })
                    .collect();

                for h in handles {
                    h.join().unwrap();
                }

                // Report metrics
                let total_conflicts = conflict_count.load(Ordering::Relaxed);
                let mut all_latencies: Vec<Duration> = latencies
                    .iter()
                    .flat_map(|m| m.lock().unwrap().clone())
                    .collect();
                all_latencies.sort();

                let p50 = percentile(&all_latencies, 50.0);
                let p95 = percentile(&all_latencies, 95.0);
                let p99 = percentile(&all_latencies, 99.0);
                let max = all_latencies.last().copied().unwrap_or(Duration::ZERO);

                eprintln!(
                    "[FrankenSQLite {n_threads}t] conflicts={total_conflicts}, p50={:?}, p95={:?}, p99={:?}, max={:?}",
                    p50, p95, p99, max
                );

                // Print phase timing report from group commit metrics
                let metrics = fsqlite_wal::GLOBAL_CONSOLIDATION_METRICS.snapshot();
                let has_phase_metrics = metrics.total_commits() > 0;
                let phase_timing_report =
                    has_phase_metrics.then(|| metrics.phase_timing_report());
                if has_phase_metrics {
                    eprintln!(
                        "[FrankenSQLite {n_threads}t wal split] flusher_lock_wait_total={}us, wal_service_total={}us, wal_backend_lock_wait_p99={}us, wal_append_p99={}us, wal_sync_p99={}us, phase_b_p99={}us, lock_topology_limited={}, wakes={{notify:{}, timeout:{}, takeover:{}, failed_epoch:{}, busy_retry:{}}}",
                        metrics.flusher_lock_wait_us_total(),
                        metrics.wal_service_us_total(),
                        metrics.hist_wal_backend_lock_wait.p99,
                        metrics.hist_wal_append.p99,
                        metrics.hist_wal_sync.p99,
                        metrics.hist_phase_b.p99,
                        metrics.is_lock_topology_limited(),
                        metrics.wake_reasons.notify,
                        metrics.wake_reasons.timeout,
                        metrics.wake_reasons.flusher_takeover,
                        metrics.wake_reasons.failed_epoch,
                        metrics.wake_reasons.busy_retry,
                    );
                    eprintln!(
                        "[FrankenSQLite {n_threads}t phase timing]\n{}",
                        phase_timing_report
                            .as_deref()
                            .unwrap_or("phase timing unavailable")
                    );
                }
                maybe_write_persistent_phase_capture(&PersistentPhaseCaptureSample {
                    schema_version: PERSISTENT_PHASE_CAPTURE_SAMPLE_SCHEMA_V1,
                    timestamp_unix_ms: unix_timestamp_ms(),
                    benchmark_group: format!("{label}/frankensqlite_concurrent_persistent"),
                    engine: "fsqlite_mvcc",
                    counter_name: "conflicts",
                    counter_value: total_conflicts,
                    concurrency: n_threads,
                    rows_per_thread: ROWS_PER_THREAD,
                    total_rows,
                    latency_us: persistent_latency_summary(&all_latencies),
                    phase_metrics: has_phase_metrics.then_some(metrics.clone()),
                    phase_timing_report,
                    flusher_lock_wait_fraction_basis_points:
                        flusher_lock_wait_fraction_basis_points(&metrics),
                    lock_topology_limited: has_phase_metrics
                        .then_some(metrics.is_lock_topology_limited()),
                });
                // Reset metrics for next iteration
                fsqlite_wal::GLOBAL_CONSOLIDATION_METRICS.reset();
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

fn bench_persistent_2t(c: &mut Criterion) {
    bench_concurrent_csqlite_persistent(c, 2, "persistent_concurrent_write_2t");
}

fn bench_persistent_4t(c: &mut Criterion) {
    bench_concurrent_csqlite_persistent(c, 4, "persistent_concurrent_write_4t");
}

fn bench_persistent_8t(c: &mut Criterion) {
    bench_concurrent_csqlite_persistent(c, 8, "persistent_concurrent_write_8t");
}

fn bench_persistent_16t(c: &mut Criterion) {
    bench_concurrent_csqlite_persistent(c, 16, "persistent_concurrent_write_16t");
}

criterion_group!(
    name = persistent_concurrent_write;
    config = criterion_config();
    targets = bench_persistent_2t, bench_persistent_4t, bench_persistent_8t, bench_persistent_16t
);
criterion_main!(persistent_concurrent_write);
