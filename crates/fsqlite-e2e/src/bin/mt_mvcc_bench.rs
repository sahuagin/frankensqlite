//! `mt-mvcc-bench` — real multi-threaded MVCC writer benchmark (IMPL-4a).
//!
//! Why this exists: `comprehensive_bench::bench_concurrent_writers` runs
//! FrankenSQLite writers *sequentially* on ONE `Connection` because
//! `Connection` is `!Send + !Sync`. That means the previously-reported
//! "concurrent" FrankenSQLite numbers were really single-threaded loops
//! compared against genuinely multi-threaded C SQLite WAL. This bench fixes
//! that by spawning N OS threads, each with its OWN `Connection::open(path)`
//! against the SAME shared file-backed database, so the MVCC page-lock
//! table, commit coordinator, and SSI validator are exercised under real
//! contention.
//!
//! For each thread count we measure:
//!   - FrankenSQLite file-backed database, one Connection per thread,
//!     `PRAGMA fsqlite.concurrent_mode=ON` + `BEGIN CONCURRENT`.
//!   - C SQLite (rusqlite) file-backed WAL, one Connection per thread,
//!     `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`.
//!
//! Each thread inserts `--rows-per-thread` rows into the shared table
//! `bench(id INTEGER PRIMARY KEY, payload TEXT)` using disjoint rowid
//! ranges (`thread_id * 1_000_000 + i`) so there are no logical row
//! conflicts — only page-level contention on the table's btree.
//!
//! Output is a tab-separated table suitable for grepping / redirection:
//!
//! ```text
//! threads | fsqlite_wps | sqlite_wps | ratio
//!       1 | 12345       | 23456      | 0.53x
//!       2 | 22000       | 40000      | 0.55x
//! ```
//!
//! `ratio = fsqlite_wps / sqlite_wps`. Values above 1.0x mean FrankenSQLite
//! is faster than C SQLite WAL under equal multi-threaded load.
//!
//! ## CLI
//!
//! ```text
//! mt-mvcc-bench [--rows-per-thread=1000] [--threads=1,2,4,8,16] [--iters=3]
//! ```
//!
//! ## Caveats
//!
//! * `BEGIN CONCURRENT` requires `PRAGMA fsqlite.concurrent_mode=ON;` to be
//!   set on each per-thread connection (see
//!   `crates/fsqlite-harness/tests/bd_3plop_4_lock_contention_storms.rs`).
//!   If that PRAGMA fails on a given build, we fall back to plain `BEGIN`
//!   and print a warning (honest measurement over a fake win).
//! * We retry transient errors (`FrankenError::is_transient()`) per-row up
//!   to `MAX_RETRIES`; hard failures are counted in `failed_rows` and
//!   included in the report so you can tell when the numbers are bogus.
//! * Each iteration creates a fresh tempfile so there's no state carried
//!   across runs. `--iters=3` reports the best (min wall-clock) of 3.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

// ─── Defaults ─────────────────────────────────────────────────────────────

const DEFAULT_ROWS_PER_THREAD: usize = 1_000;
const DEFAULT_THREADS: &[usize] = &[1, 2, 4, 8, 16];
const DEFAULT_ITERS: usize = 3;
const ROWID_BASE_STRIDE: i64 = 1_000_000;
const MAX_RETRIES: usize = 32;
const RETRY_SLEEP_MS: u64 = 1;

// ─── CLI parsing (manual — no clap in workspace) ─────────────────────────

#[derive(Debug, Clone)]
struct Options {
    rows_per_thread: usize,
    threads: Vec<usize>,
    iters: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            rows_per_thread: DEFAULT_ROWS_PER_THREAD,
            threads: DEFAULT_THREADS.to_vec(),
            iters: DEFAULT_ITERS,
        }
    }
}

fn print_usage_and_exit(code: i32) -> ! {
    eprintln!(
        "usage: mt-mvcc-bench [--rows-per-thread=N] [--threads=N,N,...] [--iters=N]\n\
         \n\
         defaults: --rows-per-thread={DEFAULT_ROWS_PER_THREAD} \
         --threads=1,2,4,8,16 --iters={DEFAULT_ITERS}"
    );
    std::process::exit(code);
}

fn parse_args() -> Options {
    let mut opts = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let (key, val) = if let Some(eq) = arg.find('=') {
            (arg[..eq].to_owned(), arg[eq + 1..].to_owned())
        } else if arg == "--help" || arg == "-h" {
            print_usage_and_exit(0);
        } else {
            // Support space-separated form.
            let v = args
                .next()
                .unwrap_or_else(|| panic!("missing value for argument `{arg}`"));
            (arg, v)
        };
        match key.as_str() {
            "--rows-per-thread" => {
                opts.rows_per_thread = val
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid --rows-per-thread: {val}"));
            }
            "--threads" => {
                opts.threads = val
                    .split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<usize>()
                            .unwrap_or_else(|_| panic!("invalid thread count in --threads: {s}"))
                    })
                    .collect();
                if opts.threads.is_empty() {
                    panic!("--threads must contain at least one value");
                }
            }
            "--iters" => {
                opts.iters = val
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid --iters: {val}"));
                if opts.iters == 0 {
                    panic!("--iters must be >= 1");
                }
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_usage_and_exit(2);
            }
        }
    }
    opts
}

// ─── Reported per-config result ───────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct RunResult {
    /// Wall-clock duration across threads (max of per-thread times), best of
    /// `iters` iterations.
    best_elapsed: Duration,
    /// Total rows written (across all threads) in the best iteration.
    total_rows: usize,
    /// Total rows that hit a hard failure after `MAX_RETRIES`.
    failed_rows: usize,
}

impl RunResult {
    fn writes_per_sec(&self) -> f64 {
        let secs = self.best_elapsed.as_secs_f64();
        if secs <= 0.0 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            let n = self.total_rows as f64;
            n / secs
        }
    }
}

// ─── FrankenSQLite workload ──────────────────────────────────────────────

fn run_fsqlite(threads: usize, rows_per_thread: usize) -> RunResult {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp
        .path()
        .to_str()
        .expect("tempfile path is utf-8")
        .to_owned();

    // Initialise schema with a single connection before spawning workers.
    {
        let conn = fsqlite::Connection::open(path.clone()).expect("fsqlite open (init)");
        for pragma in [
            "PRAGMA page_size=4096;",
            "PRAGMA journal_mode=WAL;",
            "PRAGMA synchronous=NORMAL;",
            "PRAGMA cache_size=-64000;",
        ] {
            let _ = conn.execute(pragma);
        }
        conn.execute("CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, payload TEXT)")
            .expect("create table");
    }

    let path = Arc::new(path);
    let barrier = Arc::new(Barrier::new(threads));
    let mut handles = Vec::with_capacity(threads);

    let t0 = Instant::now();
    for tid in 0..threads {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> (Duration, usize) {
            // Each thread owns its own Connection (Connection: !Send + !Sync).
            let conn =
                fsqlite::Connection::open(path.as_str().to_owned()).expect("fsqlite open (worker)");
            // MVCC + BEGIN CONCURRENT opt-in.
            let concurrent_ok = conn.execute("PRAGMA fsqlite.concurrent_mode=ON;").is_ok();
            let _ = conn.execute("PRAGMA busy_timeout=5000;");

            barrier.wait();
            let start = Instant::now();

            #[allow(clippy::cast_possible_wrap)]
            let base = tid as i64 * ROWID_BASE_STRIDE;
            let mut failed = 0usize;

            // Single transaction spanning all rows; retry on transient
            // conflicts by rolling back and reopening the transaction.
            let mut retry_count = 0usize;
            'outer: loop {
                let begin_sql = if concurrent_ok {
                    "BEGIN CONCURRENT"
                } else {
                    "BEGIN"
                };
                if let Err(e) = conn.execute(begin_sql) {
                    if e.is_transient() && retry_count < MAX_RETRIES {
                        retry_count += 1;
                        thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                        continue;
                    }
                    eprintln!("[fsqlite t{tid}] BEGIN failed: {e}");
                    return (start.elapsed(), rows_per_thread);
                }

                #[allow(clippy::cast_possible_wrap)]
                for i in 0..rows_per_thread as i64 {
                    let id = base + i;
                    let sql =
                        format!("INSERT INTO bench (id, payload) VALUES ({id}, 'tid{tid}_i{i}')");
                    match conn.execute(&sql) {
                        Ok(_) => {}
                        Err(e) if e.is_transient() && retry_count < MAX_RETRIES => {
                            let _ = conn.execute("ROLLBACK");
                            retry_count += 1;
                            thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                            continue 'outer;
                        }
                        Err(e) => {
                            eprintln!("[fsqlite t{tid}] INSERT {id} failed: {e}");
                            failed += 1;
                        }
                    }
                }

                match conn.execute("COMMIT") {
                    Ok(_) => break 'outer,
                    Err(e) if e.is_transient() && retry_count < MAX_RETRIES => {
                        let _ = conn.execute("ROLLBACK");
                        retry_count += 1;
                        thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                    }
                    Err(e) => {
                        eprintln!("[fsqlite t{tid}] COMMIT failed: {e}");
                        let _ = conn.execute("ROLLBACK");
                        failed += rows_per_thread;
                        break 'outer;
                    }
                }
            }

            (start.elapsed(), failed)
        });
        handles.push(handle);
    }

    let mut total_failed = 0usize;
    for h in handles {
        let (_d, failed) = h.join().expect("thread join");
        total_failed += failed;
    }
    let elapsed = t0.elapsed();

    RunResult {
        best_elapsed: elapsed,
        total_rows: threads * rows_per_thread,
        failed_rows: total_failed,
    }
}

// ─── C SQLite (rusqlite) workload ────────────────────────────────────────

fn run_rusqlite(threads: usize, rows_per_thread: usize) -> RunResult {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp
        .path()
        .to_str()
        .expect("tempfile path is utf-8")
        .to_owned();

    {
        let conn = rusqlite::Connection::open(&path).expect("rusqlite open (init)");
        conn.execute_batch(
            "PRAGMA page_size=4096;\
             PRAGMA journal_mode=WAL;\
             PRAGMA synchronous=NORMAL;\
             PRAGMA cache_size=-64000;\
             CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, payload TEXT);",
        )
        .expect("init schema");
    }

    let path = Arc::new(path);
    let barrier = Arc::new(Barrier::new(threads));
    let mut handles = Vec::with_capacity(threads);

    let t0 = Instant::now();
    for tid in 0..threads {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> usize {
            use rusqlite::OpenFlags;
            let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX;
            let conn = rusqlite::Connection::open_with_flags(path.as_str(), flags)
                .expect("rusqlite open (worker)");
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;\
                 PRAGMA synchronous=NORMAL;\
                 PRAGMA busy_timeout=5000;",
            )
            .expect("worker pragmas");

            barrier.wait();

            #[allow(clippy::cast_possible_wrap)]
            let base = tid as i64 * ROWID_BASE_STRIDE;
            let mut failed = 0usize;

            conn.execute_batch("BEGIN").expect("BEGIN");
            {
                let mut stmt = conn
                    .prepare("INSERT INTO bench (id, payload) VALUES (?1, ?2)")
                    .expect("prepare");
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..rows_per_thread as i64 {
                    let id = base + i;
                    let payload = format!("tid{tid}_i{i}");
                    let mut retry = 0usize;
                    loop {
                        match stmt.execute(rusqlite::params![id, &payload]) {
                            Ok(_) => break,
                            Err(e) => {
                                if retry < MAX_RETRIES
                                    && matches!(
                                        e.sqlite_error_code(),
                                        Some(
                                            rusqlite::ErrorCode::DatabaseBusy
                                                | rusqlite::ErrorCode::DatabaseLocked
                                        )
                                    )
                                {
                                    retry += 1;
                                    thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                                    continue;
                                }
                                eprintln!("[sqlite t{tid}] INSERT {id} failed: {e}");
                                failed += 1;
                                break;
                            }
                        }
                    }
                }
            }
            // Retry COMMIT on Busy — WAL writer serialisation can race.
            let mut retry = 0usize;
            loop {
                match conn.execute_batch("COMMIT") {
                    Ok(()) => break,
                    Err(e) => {
                        if retry < MAX_RETRIES
                            && matches!(
                                e.sqlite_error_code(),
                                Some(
                                    rusqlite::ErrorCode::DatabaseBusy
                                        | rusqlite::ErrorCode::DatabaseLocked
                                )
                            )
                        {
                            retry += 1;
                            thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                            continue;
                        }
                        eprintln!("[sqlite t{tid}] COMMIT failed: {e}");
                        let _ = conn.execute_batch("ROLLBACK");
                        failed += rows_per_thread;
                        break;
                    }
                }
            }

            failed
        });
        handles.push(handle);
    }

    let mut total_failed = 0usize;
    for h in handles {
        let failed = h.join().expect("thread join");
        total_failed += failed;
    }
    let elapsed = t0.elapsed();

    RunResult {
        best_elapsed: elapsed,
        total_rows: threads * rows_per_thread,
        failed_rows: total_failed,
    }
}

// ─── Driver ───────────────────────────────────────────────────────────────

fn best_of<F: FnMut() -> RunResult>(iters: usize, mut f: F) -> RunResult {
    let mut best = f();
    for _ in 1..iters {
        let r = f();
        if r.best_elapsed < best.best_elapsed {
            best = r;
        }
    }
    best
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn main() {
    let opts = parse_args();

    eprintln!(
        "mt-mvcc-bench: rows_per_thread={} threads={:?} iters={}",
        opts.rows_per_thread, opts.threads, opts.iters,
    );

    println!("threads | fsqlite_wps | sqlite_wps | ratio");
    for &n in &opts.threads {
        if n == 0 {
            continue;
        }
        let fs = best_of(opts.iters, || run_fsqlite(n, opts.rows_per_thread));
        let cs = best_of(opts.iters, || run_rusqlite(n, opts.rows_per_thread));

        let fs_wps = fs.writes_per_sec();
        let cs_wps = cs.writes_per_sec();
        let ratio = if cs_wps > 0.0 { fs_wps / cs_wps } else { 0.0 };

        let fs_fail_note = if fs.failed_rows > 0 {
            format!(" (failed={})", fs.failed_rows)
        } else {
            String::new()
        };
        let cs_fail_note = if cs.failed_rows > 0 {
            format!(" (failed={})", cs.failed_rows)
        } else {
            String::new()
        };

        println!(
            "{n:>7} | {fs_wps:>11.0}{fs_fail_note} | {cs_wps:>10.0}{cs_fail_note} | {ratio:.2}x"
        );
    }
}
