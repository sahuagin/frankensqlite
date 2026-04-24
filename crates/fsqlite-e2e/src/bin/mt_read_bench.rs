//! `mt-read-bench` — multi-threaded read-heavy SELECT-by-rowid benchmark.
//!
//! Seeds a shared file-backed DB with N rows, then spawns T threads each
//! running M prepared `SELECT payload FROM bench WHERE id = ?1` probes
//! against a disjoint rowid range. Reports reads/sec at 1/2/4/8 threads for
//! FrankenSQLite vs rusqlite (C SQLite WAL).
//!
//! Motivation: bd-...'s pinned-read + rowid-prepared-lookup wins
//! (d9c410bb, 7e4a5409, 6438b35c, b86cd4e6) should carry the read path now
//! that `mt_mvcc_bench` has proven write-side parity. This bench isolates
//! reads so any remaining read-side gap is visible.
//!
//! Usage:
//!   cargo run --release -p fsqlite-e2e --bin mt-read-bench -- \
//!       [--rows=10000] [--reads-per-thread=50000] [--threads=1,2,4,8]
//!
//! Output: one row per thread count, pipe-separated, suitable for piping
//! into a markdown table or jq.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

const DEFAULT_ROWS: i64 = 10_000;
const DEFAULT_READS_PER_THREAD: usize = 50_000;
const DEFAULT_THREADS: &[usize] = &[1, 2, 4, 8];
const PAYLOAD_SIZE: usize = 64;

fn main() {
    let mut rows = DEFAULT_ROWS;
    let mut reads_per_thread = DEFAULT_READS_PER_THREAD;
    let mut threads_opt: Vec<usize> = DEFAULT_THREADS.to_vec();

    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--rows=") {
            rows = v.parse().expect("--rows");
        } else if let Some(v) = arg.strip_prefix("--reads-per-thread=") {
            reads_per_thread = v.parse().expect("--reads-per-thread");
        } else if let Some(v) = arg.strip_prefix("--threads=") {
            threads_opt = v
                .split(',')
                .map(|s| s.trim().parse().expect("--threads"))
                .collect();
        }
    }

    eprintln!(
        "mt-read-bench: rows={rows} reads_per_thread={reads_per_thread} threads={threads_opt:?}"
    );

    println!("threads | fs_rps       | sq_rps       | ratio");
    println!("--------|--------------|--------------|--------");
    for &t in &threads_opt {
        if t == 0 {
            continue;
        }
        let fs = run_fsqlite(t, rows, reads_per_thread);
        let sq = run_rusqlite(t, rows, reads_per_thread);
        #[allow(clippy::cast_precision_loss)]
        let ratio = fs / sq.max(1.0);
        println!(
            "{t:>7} | {fs:>12.0} | {sq:>12.0} | {ratio:>5.2}x",
            fs = fs,
            sq = sq
        );
    }
}

#[allow(clippy::cast_precision_loss)]
fn run_fsqlite(n_threads: usize, rows: i64, reads_per_thread: usize) -> f64 {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path: String = tmp.path().to_string_lossy().into_owned();
    drop(tmp);

    // Seed
    {
        let conn = fsqlite::Connection::open(path.clone()).expect("fsqlite open seed");
        let _ = conn.execute("PRAGMA fsqlite.concurrent_mode=ON;");
        conn.execute("CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, payload TEXT)")
            .expect("create");
        conn.execute("BEGIN").expect("begin");
        let stmt = conn
            .prepare("INSERT INTO bench (id, payload) VALUES (?1, ?2)")
            .expect("prepare insert");
        let payload = "x".repeat(PAYLOAD_SIZE);
        for id in 1..=rows {
            let params = [
                fsqlite::SqliteValue::Integer(id),
                fsqlite::SqliteValue::Text(payload.clone().into()),
            ];
            stmt.execute_with_params(&params).expect("insert");
        }
        conn.execute("COMMIT").expect("commit");
    }

    let path = Arc::new(path);
    let barrier = Arc::new(Barrier::new(n_threads));
    let mut handles = Vec::with_capacity(n_threads);
    let t0 = Instant::now();
    for tid in 0..n_threads {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let conn = fsqlite::Connection::open(path.as_str().to_owned()).expect("fsqlite open");
            let _ = conn.execute("PRAGMA fsqlite.concurrent_mode=ON;");
            let stmt = conn
                .prepare("SELECT payload FROM bench WHERE id = ?1")
                .expect("prepare select");
            barrier.wait();
            let mut state = 0x0102_0304_0506_0708_u64 ^ (tid as u64).wrapping_mul(0x9e37);
            for _ in 0..reads_per_thread {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                #[allow(clippy::cast_possible_wrap)]
                let id = ((state % rows as u64) + 1) as i64;
                let params = [fsqlite::SqliteValue::Integer(id)];
                let _ = stmt.query_with_params(&params).expect("query");
            }
        }));
    }
    for h in handles {
        h.join().expect("join");
    }
    let elapsed = t0.elapsed().as_secs_f64();
    (n_threads * reads_per_thread) as f64 / elapsed
}

#[allow(clippy::cast_precision_loss)]
fn run_rusqlite(n_threads: usize, rows: i64, reads_per_thread: usize) -> f64 {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path: String = tmp.path().to_string_lossy().into_owned();
    drop(tmp);

    // Seed
    {
        let conn = rusqlite::Connection::open(&path).expect("sqlite open seed");
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; \
             PRAGMA synchronous=NORMAL; \
             PRAGMA busy_timeout=5000; \
             CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, payload TEXT);",
        )
        .expect("pragmas");
        conn.execute_batch("BEGIN").expect("begin");
        let mut stmt = conn
            .prepare("INSERT INTO bench (id, payload) VALUES (?1, ?2)")
            .expect("prepare insert");
        let payload = "x".repeat(PAYLOAD_SIZE);
        for id in 1..=rows {
            stmt.execute(rusqlite::params![id, payload])
                .expect("insert");
        }
        drop(stmt);
        conn.execute_batch("COMMIT").expect("commit");
    }

    let path = Arc::new(path);
    let barrier = Arc::new(Barrier::new(n_threads));
    let mut handles = Vec::with_capacity(n_threads);
    let t0 = Instant::now();
    for tid in 0..n_threads {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let conn = rusqlite::Connection::open(path.as_str()).expect("sqlite open");
            conn.execute_batch("PRAGMA busy_timeout=5000;").ok();
            let mut stmt = conn
                .prepare("SELECT payload FROM bench WHERE id = ?1")
                .expect("prepare select");
            barrier.wait();
            let mut state = 0x0102_0304_0506_0708_u64 ^ (tid as u64).wrapping_mul(0x9e37);
            for _ in 0..reads_per_thread {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                #[allow(clippy::cast_possible_wrap)]
                let id = ((state % rows as u64) + 1) as i64;
                let _: Option<String> = stmt.query_row(rusqlite::params![id], |r| r.get(0)).ok();
            }
        }));
    }
    for h in handles {
        h.join().expect("join");
    }
    let elapsed = t0.elapsed().as_secs_f64();
    (n_threads * reads_per_thread) as f64 / elapsed
}
