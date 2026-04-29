#![recursion_limit = "256"]

//! Comprehensive FrankenSQLite vs C SQLite benchmark.
//!
//! Measures insertion throughput across multiple dimensions:
//!
//! **Row counts:** 100, 1K, 10K, 100K
//! **Record sizes:** tiny (1 col), small (3 cols), medium (6 cols), large (10 cols with ~500B text)
//! **Transaction strategies:** autocommit, batched (1K per txn), single txn
//! **Concurrency:** single writer, 2/4/8 concurrent writers (C SQLite WAL vs FrankenSQLite MVCC)
//! **Read-after-write:** full scan, point lookup, range scan, COUNT(*), indexed lookup
//!
//! Usage:
//!   cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
//!   cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick
//!   cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --filter insert

use std::collections::BTreeMap;
use std::io::Write as _;
use std::sync::{Arc, Barrier, mpsc};
use std::time::{Duration, Instant, SystemTime};

use asupersync::runtime::{BlockingTaskHandle, Runtime, RuntimeBuilder};
use serde::Serialize;

// ─── Configuration ─────────────────────────────────────────────────────

const WARMUP_ITERS: usize = 2;
const MIN_ITERS: usize = 3;
const MAX_ITERS: usize = 10;
const TARGET_DURATION: Duration = Duration::from_secs(5);

const ROW_COUNTS: &[usize] = &[100, 1_000, 10_000, 100_000];
const ROW_COUNTS_QUICK: &[usize] = &[100, 1_000, 10_000];

const CONCURRENT_THREAD_COUNTS: &[usize] = &[2, 4, 8];
const CONCURRENT_ROWS_PER_THREAD: usize = 1_000;
const CONCURRENT_RANGE_SIZE: i64 = 1_000_000;
const JSON_REPORT_SCHEMA_V3: &str = "fsqlite-e2e.comprehensive-bench-report.v3";
const CI_REGRESSION_GATE_SCHEMA_V2: &str = "fsqlite-e2e.comprehensive-bench-ci-regression-gate.v2";
const CI_REGRESSION_GATE_BEAD_ID: &str = "bd-m4tju";
const CI_REGRESSION_BASELINE_BEAD_ID: &str = "bd-0winn";
const CI_REGRESSION_BASELINE_AVG_RATIO: f64 = 2.74;
const CI_REGRESSION_GATE_STATUS_RICH_SCORECARD: &str = "rich_scorecard_schema_ready";
const CI_REGRESSION_GATE_THRESHOLD_SOURCE: &str =
    "bd-d4m5k rich scorecard: primary gate is per_category_weighted.score";
const CI_PRIMARY_SCORE_MAX_REGRESSION_PCT: f64 = 0.03;
const CI_GEOMEAN_MAX_REGRESSION_PCT: f64 = 0.05;
const CI_CATEGORY_GEOMEAN_MAX_REGRESSION_PCT: f64 = 0.10;
const CI_P90_MAX_REGRESSION_PCT: f64 = 0.15;
const CONCURRENT_WRITERS_SECTION_TITLE: &str =
    "Concurrent Writers — C SQLite WAL vs FrankenSQLite MVCC";

// ─── Record size definitions ───────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum RecordSize {
    Tiny,
    Small,
    Medium,
    Large,
}

impl RecordSize {
    const ALL: &[Self] = &[Self::Tiny, Self::Small, Self::Medium, Self::Large];

    const fn name(self) -> &'static str {
        match self {
            Self::Tiny => "tiny_1col",
            Self::Small => "small_3col",
            Self::Medium => "medium_6col",
            Self::Large => "large_10col",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Tiny => "1 col (INTEGER PK only)",
            Self::Small => "3 cols (~30B: id, name, value)",
            Self::Medium => "6 cols (~180B: id, name, email, bio, category, score)",
            Self::Large => "10 cols (~600B: includes long text fields)",
        }
    }

    fn create_table_sql(self) -> &'static str {
        match self {
            Self::Tiny => "CREATE TABLE bench (id INTEGER PRIMARY KEY)",
            Self::Small => {
                "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT NOT NULL, value REAL NOT NULL)"
            }
            Self::Medium => {
                "CREATE TABLE bench (\
                id INTEGER PRIMARY KEY, \
                first_name TEXT NOT NULL, \
                last_name TEXT NOT NULL, \
                email TEXT NOT NULL, \
                bio TEXT NOT NULL, \
                score INTEGER NOT NULL\
            )"
            }
            Self::Large => {
                "CREATE TABLE bench (\
                id INTEGER PRIMARY KEY, \
                first_name TEXT NOT NULL, \
                last_name TEXT NOT NULL, \
                email TEXT NOT NULL, \
                department TEXT NOT NULL, \
                title TEXT NOT NULL, \
                bio TEXT NOT NULL, \
                address TEXT NOT NULL, \
                notes TEXT NOT NULL, \
                score INTEGER NOT NULL\
            )"
            }
        }
    }

    fn insert_sql_csqlite(self) -> &'static str {
        match self {
            Self::Tiny => "INSERT INTO bench VALUES (?1)",
            Self::Small => "INSERT INTO bench VALUES (?1, ('user_' || ?1), (?1 * 0.137))",
            Self::Medium => {
                "INSERT INTO bench VALUES (\
                ?1, \
                ('Alice_' || ?1), \
                ('Smith_' || ?1), \
                ('user' || ?1 || '@example.com'), \
                ('Bio text for user number ' || ?1 || '. This is a medium-length description that adds some realistic payload to each row in the database.'), \
                (?1 * 7)\
            )"
            }
            Self::Large => {
                "INSERT INTO bench VALUES (\
                ?1, \
                ('FirstName_' || ?1), \
                ('LastName_' || ?1), \
                ('employee' || ?1 || '@bigcorp.example.com'), \
                ('Engineering_Dept_' || (?1 % 20)), \
                ('Senior Software Engineer Level ' || (?1 % 5)), \
                ('This is the biography for employee number ' || ?1 || '. They have been working at the company for many years and have contributed to numerous projects across multiple teams. Their expertise spans distributed systems, database internals, and performance optimization. They are known for their thorough code reviews and mentorship of junior engineers.'), \
                (?1 || ' Technology Park, Building ' || (?1 % 50) || ', Suite ' || (?1 % 200) || ', Innovation City, CA 94000'), \
                ('Internal notes: Employee ' || ?1 || ' - Performance rating: Exceeds Expectations. Last review date: 2026-01-15. Next review: 2026-07-15. Skills: Rust, C++, SQL, distributed systems, leadership.'), \
                (?1 * 13)\
                )"
            }
        }
    }
}

// ─── Measurement infrastructure ────────────────────────────────────────

#[allow(dead_code)]
#[derive(Clone)]
struct Measurement {
    label: String,
    durations: Vec<Duration>,
    row_count: usize,
}

#[allow(dead_code)]
impl Measurement {
    fn mean(&self) -> Duration {
        let total: Duration = self.durations.iter().sum();
        total / u32::try_from(self.durations.len()).unwrap_or(1)
    }

    fn median(&self) -> Duration {
        let mut sorted: Vec<Duration> = self.durations.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    fn min(&self) -> Duration {
        self.durations.iter().copied().min().unwrap_or_default()
    }

    fn stddev(&self) -> Duration {
        let mean = self.mean().as_nanos() as f64;
        let variance = self
            .durations
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / self.durations.len() as f64;
        Duration::from_nanos(variance.sqrt() as u64)
    }

    fn rows_per_sec(&self) -> f64 {
        let secs = self.median().as_secs_f64();
        if secs == 0.0 {
            return 0.0;
        }
        self.row_count as f64 / secs
    }

    fn us_per_row(&self) -> f64 {
        let us = self.median().as_secs_f64() * 1_000_000.0;
        if self.row_count == 0 {
            return 0.0;
        }
        us / self.row_count as f64
    }

    fn percentile(&self, pct: f64) -> Duration {
        let mut sorted: Vec<Duration> = self.durations.clone();
        sorted.sort();
        let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).ceil() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    fn p95(&self) -> Duration {
        self.percentile(95.0)
    }

    fn p99(&self) -> Duration {
        self.percentile(99.0)
    }

    fn cv_percent(&self) -> f64 {
        let mean_ns = self.mean().as_nanos() as f64;
        if mean_ns == 0.0 {
            return 0.0;
        }
        let stddev_ns = self.stddev().as_nanos() as f64;
        (stddev_ns / mean_ns) * 100.0
    }

    fn iter_count(&self) -> usize {
        self.durations.len()
    }
}

fn measure<F: FnMut()>(label: &str, row_count: usize, mut f: F) -> Measurement {
    // Warmup
    for w in 0..WARMUP_ITERS {
        eprint!("\r    [{label}] warmup {}/{WARMUP_ITERS}...", w + 1);
        f();
    }

    let mut durations = Vec::new();
    let mut total_elapsed = Duration::ZERO;

    for iter in 0..MAX_ITERS {
        eprint!(
            "\r    [{label}] iter {}/{MAX_ITERS} (total: {:.1}s)    ",
            iter + 1,
            total_elapsed.as_secs_f64()
        );
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        durations.push(elapsed);
        total_elapsed += elapsed;

        if iter >= MIN_ITERS && total_elapsed >= TARGET_DURATION {
            break;
        }
    }
    eprint!("\r{:80}\r", ""); // Clear progress line.

    Measurement {
        label: label.to_string(),
        durations,
        row_count,
    }
}

// ─── BusySnapshot / Busy retry helpers ─────────────────────────────────
//
// FrankenSQLite's MVCC can return `BusySnapshot` or `Busy` when a write
// races against another writer/snapshot. These are transient and must be
// retried with backoff, analogous to SQLITE_BUSY under WAL.  The bench
// harness uses a bounded exponential backoff so spurious contention on
// shared structures (e.g. the single-connection cache, the pager, or
// transient snapshot conflicts) does not turn into a hard panic.

/// Maximum number of retry attempts per mutation.
const BENCH_BUSY_MAX_RETRIES: u32 = 32;
/// Starting backoff in microseconds (doubles each attempt, capped at
/// ~100ms via the `min(10)` shift clamp).
const BENCH_BUSY_BACKOFF_US: u64 = 100;

fn is_busy_like(err: &fsqlite::FrankenError) -> bool {
    matches!(
        err,
        fsqlite::FrankenError::BusySnapshot { .. }
            | fsqlite::FrankenError::Busy
            | fsqlite::FrankenError::BusyRecovery
            | fsqlite::FrankenError::DatabaseLocked { .. }
            | fsqlite::FrankenError::LockFailed { .. }
    )
}

/// Retry `op` with bounded exponential backoff while it returns a
/// busy-like error.  Returns the last error if retries are exhausted.
fn retry_on_busy<T, F>(mut op: F) -> Result<T, fsqlite::FrankenError>
where
    F: FnMut() -> Result<T, fsqlite::FrankenError>,
{
    let mut attempt: u32 = 0;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) if is_busy_like(&e) && attempt < BENCH_BUSY_MAX_RETRIES => {
                let shift = attempt.min(10);
                let wait_us = BENCH_BUSY_BACKOFF_US << shift;
                std::thread::sleep(Duration::from_micros(wait_us));
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// `conn.execute(sql)` with BusySnapshot/Busy retry.
fn fs_execute(conn: &fsqlite::Connection, sql: &str) -> usize {
    retry_on_busy(|| conn.execute(sql))
        .unwrap_or_else(|e| panic!("fsqlite execute failed after retries: {e} (sql={sql})"))
}

/// `stmt.execute_with_params(params)` with BusySnapshot/Busy retry.
fn fs_stmt_execute_with_params(
    stmt: &fsqlite::PreparedStatement<'_>,
    params: &[fsqlite::SqliteValue],
) -> usize {
    retry_on_busy(|| stmt.execute_with_params(params)).unwrap_or_else(|e| {
        panic!("fsqlite prepared execute_with_params failed after retries: {e}")
    })
}

fn collect_rusqlite_rows<P: rusqlite::Params>(
    stmt: &mut rusqlite::Statement<'_>,
    params: P,
) -> rusqlite::Result<Vec<Vec<rusqlite::types::Value>>> {
    let col_count = stmt.column_count();
    stmt.query_map(params, move |row| {
        let mut values = Vec::with_capacity(col_count);
        for idx in 0..col_count {
            let value = row.get(idx).unwrap_or(rusqlite::types::Value::Null);
            values.push(value);
        }
        Ok(values)
    })?
    .collect::<Result<Vec<_>, _>>()
}

struct BenchTask<T> {
    handle: BlockingTaskHandle,
    result_rx: mpsc::Receiver<Result<T, String>>,
}

impl<T> BenchTask<T> {
    fn wait(self) -> T {
        self.handle.wait();
        match self.result_rx.recv() {
            Ok(Ok(value)) => value,
            Ok(Err(message)) => panic!("{message}"),
            Err(err) => panic!("benchmark worker exited without reporting a result: {err}"),
        }
    }
}

fn spawn_bench_task<T, F>(runtime: &Runtime, task: F) -> BenchTask<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (result_tx, result_rx) = mpsc::channel();
    let handle = runtime
        .spawn_blocking(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task))
                .map_err(panic_payload_to_string);
            let _ = result_tx.send(outcome);
        })
        .expect("comprehensive benchmark runtime must configure a blocking pool");
    BenchTask { handle, result_rx }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(_) => "non-string panic payload".to_owned(),
        },
    }
}

// ─── PRAGMA helpers ────────────────────────────────────────────────────

fn apply_pragmas_csqlite(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "PRAGMA page_size = 4096;\
         PRAGMA journal_mode = WAL;\
         PRAGMA synchronous = NORMAL;\
         PRAGMA cache_size = -64000;",
    )
    .ok();
}

const FSQLITE_BENCHMARK_PRAGMAS: &[&str] = &[
    "PRAGMA page_size = 4096;",
    "PRAGMA journal_mode = WAL;",
    "PRAGMA synchronous = NORMAL;",
    "PRAGMA cache_size = -64000;",
    // Comprehensive benchmark workloads compare SQLite-compatible query and
    // write paths. They never issue `FOR SYSTEM_TIME` queries, so keep the
    // optional in-memory snapshot ring out of the hot path.
    "PRAGMA fsqlite_capture_time_travel_snapshots=false;",
];

fn apply_pragmas_fsqlite(conn: &fsqlite::Connection) {
    for pragma in FSQLITE_BENCHMARK_PRAGMAS {
        let _ = conn.execute(pragma);
    }
    // Opt-in LAB_UNSAFE write-merge mode for A/B perf measurement of the
    // SSI e-process skip gate. The gate is safe to leave on: under the
    // benchmark's pivot-free workloads, SSI validation is the dominant
    // constant-time overhead per commit.
    if std::env::var("FSQLITE_BENCH_LAB_UNSAFE")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        let _ = conn.execute("PRAGMA fsqlite.write_merge = LAB_UNSAFE;");
        // Tight alpha so the gate opens reasonably fast on the short
        // benchmark runs. `alpha = 1e-3` matches the default.
        let _ = conn.execute("PRAGMA fsqlite.ssi_e_process_alpha = 0.001;");
    }
}

// ─── Report formatting ────────────────────────────────────────────────

struct BenchReport {
    sections: Vec<ReportSection>,
}

struct ReportSection {
    title: String,
    description: String,
    rows: Vec<ReportRow>,
}

struct ReportRow {
    scenario: String,
    csqlite: Option<Measurement>,
    fsqlite: Option<Measurement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct CliOptions {
    quick: bool,
    filter: Option<String>,
    html_path: Option<String>,
    emit_html: bool,
    emit_timestamped_json: bool,
    json_out_path: Option<String>,
    json_stdout: bool,
    print_json_schema: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ScenarioCategory {
    ReadSingle,
    ReadAggregate,
    WriteSingle,
    WriteBulk,
    ConcurrentWriters,
    MixedOltp,
}

impl ScenarioCategory {
    const ALL: [Self; 6] = [
        Self::ReadSingle,
        Self::ReadAggregate,
        Self::WriteSingle,
        Self::WriteBulk,
        Self::ConcurrentWriters,
        Self::MixedOltp,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::ReadSingle => "read_single",
            Self::ReadAggregate => "read_aggregate",
            Self::WriteSingle => "write_single",
            Self::WriteBulk => "write_bulk",
            Self::ConcurrentWriters => "concurrent_writers",
            Self::MixedOltp => "mixed",
        }
    }

    const fn default_weight(self) -> f64 {
        match self {
            Self::ReadSingle => 0.35,
            Self::ReadAggregate => 0.15,
            Self::WriteSingle => 0.30,
            Self::WriteBulk => 0.10,
            Self::ConcurrentWriters | Self::MixedOltp => 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ReportSummaryStats {
    total_scenarios: usize,
    franken_faster: usize,
    comparable: usize,
    csqlite_faster: usize,
    avg_ratio: Option<f64>,
    average_ratio: Option<f64>,
    geomean_ratio: Option<f64>,
    median_ratio: Option<f64>,
    p90_ratio: Option<f64>,
    p99_ratio: Option<f64>,
    primary_metric: String,
    per_category: BTreeMap<String, JsonCategoryRatioStats>,
    per_category_weighted: JsonWeightedCategoryScore,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct DetectedEnvironment {
    os: Option<String>,
    arch: String,
    kernel_release: Option<String>,
    cpu_model: Option<String>,
    cpu_cores: Option<usize>,
    ram_gb: Option<f64>,
    active_toolchain: Option<String>,
    rust_version: Option<String>,
    cargo_version: Option<String>,
    git_commit_sha: Option<String>,
    git_branch: Option<String>,
    build_profile: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct JsonRunConfig {
    quick: bool,
    filter: Option<String>,
    warmup_iterations: usize,
    min_iterations: usize,
    max_iterations: usize,
    target_duration_secs: u64,
    row_counts: Vec<usize>,
    html_output_path: Option<String>,
    json_output_path: Option<String>,
    json_stdout: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonMeasurement {
    median_ms: f64,
    mean_ms: f64,
    min_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    stddev_ms: f64,
    cv_pct: f64,
    rows_per_sec: f64,
    us_per_row: f64,
    iterations: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonRow {
    scenario_id: String,
    scenario: String,
    category: String,
    csqlite: Option<JsonMeasurement>,
    fsqlite: Option<JsonMeasurement>,
    ratio_fsqlite_over_csqlite: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonCategoryRatioStats {
    n: usize,
    avg_ratio: Option<f64>,
    geomean_ratio: Option<f64>,
    median_ratio: Option<f64>,
    p90_ratio: Option<f64>,
    p99_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonWeightedCategoryScore {
    primary: bool,
    score: Option<f64>,
    weights: BTreeMap<String, f64>,
    observed_weight: f64,
    missing_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonSection {
    section_id: String,
    title: String,
    description: String,
    rows: Vec<JsonRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonBenchmarkReport {
    schema_version: String,
    generated_at_utc: String,
    total_elapsed_ms: u64,
    config: JsonRunConfig,
    environment: DetectedEnvironment,
    summary: ReportSummaryStats,
    ci_regression_gate: JsonCiRegressionGateDraft,
    sections: Vec<JsonSection>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonCiRegressionGateDraft {
    schema_version: String,
    bead_id: String,
    depends_on_bead_id: String,
    status: String,
    thresholds: JsonCiRegressionThresholdsDraft,
    observed: JsonCiRegressionObservedMetrics,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonCiRegressionThresholdsDraft {
    avg_ratio_baseline: f64,
    avg_ratio_max: Option<f64>,
    mt_p95_ratio_max: Option<f64>,
    primary_score_max_regression_pct: f64,
    geomean_max_regression_pct: f64,
    per_category_geomean_max_regression_pct: f64,
    p90_max_regression_pct: f64,
    threshold_source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JsonCiRegressionObservedMetrics {
    avg_ratio: Option<f64>,
    primary_score: Option<f64>,
    geomean_ratio: Option<f64>,
    median_ratio: Option<f64>,
    p90_ratio: Option<f64>,
    max_mt_p95_ratio: Option<f64>,
    max_mt_p95_scenario_id: Option<String>,
}

fn compute_report_summary(report: &BenchReport) -> ReportSummaryStats {
    let mut franken_faster = 0_usize;
    let mut csqlite_faster = 0_usize;
    let mut comparable = 0_usize;
    let mut ratios = Vec::new();
    let mut category_ratios: BTreeMap<ScenarioCategory, Vec<f64>> = BTreeMap::new();

    for section in &report.sections {
        for row in &section.rows {
            if let Some(ratio) = row_ratio(row) {
                ratios.push(ratio);
                category_ratios
                    .entry(categorize_scenario(&section.title, &row.scenario))
                    .or_default()
                    .push(ratio);
                if ratio < 0.95 {
                    franken_faster += 1;
                } else if ratio > 1.05 {
                    csqlite_faster += 1;
                } else {
                    comparable += 1;
                }
            }
        }
    }

    let aggregate = ratio_stats(&ratios);
    let mut per_category = BTreeMap::new();
    for category in ScenarioCategory::ALL {
        let stats = ratio_stats(
            category_ratios
                .get(&category)
                .map_or(&[][..], Vec::as_slice),
        );
        per_category.insert(category.id().to_owned(), stats);
    }
    let per_category_weighted = weighted_category_score(&per_category);

    ReportSummaryStats {
        total_scenarios: ratios.len(),
        franken_faster,
        comparable,
        csqlite_faster,
        avg_ratio: aggregate.avg_ratio,
        average_ratio: aggregate.avg_ratio,
        geomean_ratio: aggregate.geomean_ratio,
        median_ratio: aggregate.median_ratio,
        p90_ratio: aggregate.p90_ratio,
        p99_ratio: aggregate.p99_ratio,
        primary_metric: "per_category_weighted.score".to_owned(),
        per_category,
        per_category_weighted,
    }
}

fn categorize_scenario(section_title: &str, scenario: &str) -> ScenarioCategory {
    let section = section_title.to_ascii_lowercase();
    let scenario = scenario.to_ascii_lowercase();

    if section.contains("concurrent writers") {
        return ScenarioCategory::ConcurrentWriters;
    }
    if section.contains("mixed oltp") {
        return ScenarioCategory::MixedOltp;
    }
    if section.contains("transaction strategy") {
        return if scenario.contains("autocommit") {
            ScenarioCategory::WriteSingle
        } else {
            ScenarioCategory::WriteBulk
        };
    }
    if section.contains("insert") || section.contains("record size") {
        return ScenarioCategory::WriteBulk;
    }
    if section.contains("update") || section.contains("delete") {
        return ScenarioCategory::WriteSingle;
    }
    if section.contains("join") || section.contains("subquery") || section.contains("cte") {
        return if scenario.contains("group")
            || scenario.contains("having")
            || scenario.contains("count")
            || scenario.contains("sum")
            || scenario.contains("exists")
            || scenario.contains(" in subquery")
            || scenario.contains("cte")
        {
            ScenarioCategory::ReadAggregate
        } else {
            ScenarioCategory::ReadSingle
        };
    }
    if section.contains("string") {
        return if scenario.contains("group_concat") {
            ScenarioCategory::ReadAggregate
        } else {
            ScenarioCategory::ReadSingle
        };
    }
    if section.contains("read") || section.contains("query") || section.contains("select") {
        return if scenario.contains("count")
            || scenario.contains("group")
            || scenario.contains("sum")
            || scenario.contains("aggregate")
        {
            ScenarioCategory::ReadAggregate
        } else {
            ScenarioCategory::ReadSingle
        };
    }

    ScenarioCategory::ReadSingle
}

fn ratio_stats(ratios: &[f64]) -> JsonCategoryRatioStats {
    let mut sorted: Vec<f64> = ratios
        .iter()
        .copied()
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
        .collect();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    if n == 0 {
        return JsonCategoryRatioStats {
            n: 0,
            avg_ratio: None,
            geomean_ratio: None,
            median_ratio: None,
            p90_ratio: None,
            p99_ratio: None,
        };
    }

    let sum = sorted.iter().sum::<f64>();
    let log_sum = sorted.iter().map(|ratio| ratio.ln()).sum::<f64>();
    JsonCategoryRatioStats {
        n,
        avg_ratio: Some(sum / n as f64),
        geomean_ratio: Some((log_sum / n as f64).exp()),
        median_ratio: percentile_ratio(&sorted, 50.0),
        p90_ratio: percentile_ratio(&sorted, 90.0),
        p99_ratio: percentile_ratio(&sorted, 99.0),
    }
}

fn percentile_ratio(sorted: &[f64], pct: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).ceil() as usize;
    sorted.get(idx.min(sorted.len() - 1)).copied()
}

fn category_weights() -> BTreeMap<String, f64> {
    ScenarioCategory::ALL
        .into_iter()
        .map(|category| (category.id().to_owned(), category.default_weight()))
        .collect()
}

fn weighted_category_score(
    per_category: &BTreeMap<String, JsonCategoryRatioStats>,
) -> JsonWeightedCategoryScore {
    let weights = category_weights();
    let mut weighted_log_sum = 0.0_f64;
    let mut observed_weight = 0.0_f64;
    let mut missing_categories = Vec::new();

    for (category, weight) in &weights {
        match per_category
            .get(category)
            .and_then(|stats| stats.geomean_ratio)
        {
            Some(geomean) => {
                weighted_log_sum += geomean.ln() * weight;
                observed_weight += weight;
            }
            None => missing_categories.push(category.clone()),
        }
    }

    JsonWeightedCategoryScore {
        primary: true,
        score: (observed_weight > 0.0).then_some((weighted_log_sum / observed_weight).exp()),
        weights,
        observed_weight,
        missing_categories,
    }
}

fn row_ratio(row: &ReportRow) -> Option<f64> {
    let csqlite = row.csqlite.as_ref()?;
    let fsqlite = row.fsqlite.as_ref()?;
    let csqlite_nanos = csqlite.median().as_nanos();
    if csqlite_nanos == 0 {
        return None;
    }
    Some(fsqlite.median().as_nanos() as f64 / csqlite_nanos as f64)
}

fn row_p95_ratio(row: &ReportRow) -> Option<f64> {
    let csqlite = row.csqlite.as_ref()?;
    let fsqlite = row.fsqlite.as_ref()?;
    let csqlite_p95_nanos = csqlite.p95().as_nanos();
    if csqlite_p95_nanos == 0 {
        return None;
    }
    Some(fsqlite.p95().as_nanos() as f64 / csqlite_p95_nanos as f64)
}

fn max_multithread_p95_ratio(report: &BenchReport) -> (Option<f64>, Option<String>) {
    report
        .sections
        .iter()
        .filter(|section| section.title == CONCURRENT_WRITERS_SECTION_TITLE)
        .flat_map(|section| {
            let section_id = stable_slug(&section.title);
            section.rows.iter().filter_map(move |row| {
                row_p95_ratio(row).map(|ratio| {
                    let scenario_id = format!("{}__{}", section_id, stable_slug(&row.scenario));
                    (ratio, scenario_id)
                })
            })
        })
        .max_by(|(left, _), (right, _)| left.total_cmp(right))
        .map_or((None, None), |(ratio, scenario_id)| {
            (Some(ratio), Some(scenario_id))
        })
}

fn build_ci_regression_gate(
    report: &BenchReport,
    summary: &ReportSummaryStats,
) -> JsonCiRegressionGateDraft {
    let (max_mt_p95_ratio, max_mt_p95_scenario_id) = max_multithread_p95_ratio(report);
    JsonCiRegressionGateDraft {
        schema_version: CI_REGRESSION_GATE_SCHEMA_V2.to_owned(),
        bead_id: CI_REGRESSION_GATE_BEAD_ID.to_owned(),
        depends_on_bead_id: CI_REGRESSION_BASELINE_BEAD_ID.to_owned(),
        status: CI_REGRESSION_GATE_STATUS_RICH_SCORECARD.to_owned(),
        thresholds: JsonCiRegressionThresholdsDraft {
            avg_ratio_baseline: CI_REGRESSION_BASELINE_AVG_RATIO,
            avg_ratio_max: None,
            mt_p95_ratio_max: None,
            primary_score_max_regression_pct: CI_PRIMARY_SCORE_MAX_REGRESSION_PCT,
            geomean_max_regression_pct: CI_GEOMEAN_MAX_REGRESSION_PCT,
            per_category_geomean_max_regression_pct: CI_CATEGORY_GEOMEAN_MAX_REGRESSION_PCT,
            p90_max_regression_pct: CI_P90_MAX_REGRESSION_PCT,
            threshold_source: CI_REGRESSION_GATE_THRESHOLD_SOURCE.to_owned(),
        },
        observed: JsonCiRegressionObservedMetrics {
            avg_ratio: summary.avg_ratio,
            primary_score: summary.per_category_weighted.score,
            geomean_ratio: summary.geomean_ratio,
            median_ratio: summary.median_ratio,
            p90_ratio: summary.p90_ratio,
            max_mt_p95_ratio,
            max_mt_p95_scenario_id,
        },
    }
}

fn stable_slug(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_sep = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep && !slug.is_empty() {
            slug.push('-');
            last_was_sep = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "unnamed".to_owned()
    } else {
        slug
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

impl JsonMeasurement {
    fn from_measurement(measurement: &Measurement) -> Self {
        Self {
            median_ms: duration_ms(measurement.median()),
            mean_ms: duration_ms(measurement.mean()),
            min_ms: duration_ms(measurement.min()),
            p95_ms: duration_ms(measurement.p95()),
            p99_ms: duration_ms(measurement.p99()),
            stddev_ms: duration_ms(measurement.stddev()),
            cv_pct: measurement.cv_percent(),
            rows_per_sec: measurement.rows_per_sec(),
            us_per_row: measurement.us_per_row(),
            iterations: measurement.iter_count(),
        }
    }
}

impl DetectedEnvironment {
    fn detect() -> Self {
        fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
            std::process::Command::new(program)
                .args(args)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .filter(|stdout| !stdout.is_empty())
        }

        let os = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|os_release| {
                os_release.lines().find_map(|line| {
                    line.strip_prefix("PRETTY_NAME=")
                        .map(|pretty| pretty.trim_matches('"').to_owned())
                })
            });

        let (cpu_model, cpu_cores) =
            std::fs::read_to_string("/proc/cpuinfo")
                .ok()
                .map_or((None, None), |cpuinfo| {
                    let mut model = None;
                    let mut count = 0_usize;
                    for line in cpuinfo.lines() {
                        if line.starts_with("model name") {
                            if model.is_none() {
                                model = line.split(':').nth(1).map(|part| part.trim().to_owned());
                            }
                            count += 1;
                        }
                    }
                    (model, (count > 0).then_some(count))
                });

        let kernel_release = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|release| release.trim().to_owned())
            .filter(|release| !release.is_empty());

        let ram_gb = std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|meminfo| {
                meminfo.lines().find_map(|line| {
                    if !line.starts_with("MemTotal:") {
                        return None;
                    }
                    let kb_str: String = line.chars().filter(char::is_ascii_digit).collect();
                    kb_str.parse::<u64>().ok().map(|kb| kb as f64 / 1_048_576.0)
                })
            });

        let active_toolchain =
            command_stdout("rustup", &["show", "active-toolchain"]).or_else(|| {
                std::env::var("RUSTUP_TOOLCHAIN")
                    .ok()
                    .filter(|toolchain| !toolchain.is_empty())
            });
        let rust_version = command_stdout("rustc", &["--version"]);
        let cargo_version = command_stdout("cargo", &["--version"]);
        let git_commit_sha = command_stdout("git", &["rev-parse", "HEAD"]);
        let git_branch = command_stdout("git", &["branch", "--show-current"]);

        Self {
            os,
            arch: std::env::consts::ARCH.to_owned(),
            kernel_release,
            cpu_model,
            cpu_cores,
            ram_gb,
            active_toolchain,
            rust_version,
            cargo_version,
            git_commit_sha,
            git_branch,
            build_profile: "release-perf".to_owned(),
        }
    }

    fn print(&self, to_stdout: bool) {
        if let Some(os) = &self.os {
            emit_line(to_stdout, format!("  OS: {os}"));
        }
        emit_line(to_stdout, format!("  Arch: {}", self.arch));
        if let Some(kernel_release) = &self.kernel_release {
            emit_line(to_stdout, format!("  Kernel: {kernel_release}"));
        }
        if let Some(cpu_model) = &self.cpu_model {
            match self.cpu_cores {
                Some(cpu_cores) => {
                    emit_line(to_stdout, format!("  CPU: {cpu_model} ({cpu_cores} cores)"));
                }
                None => emit_line(to_stdout, format!("  CPU: {cpu_model}")),
            }
        }
        if let Some(ram_gb) = self.ram_gb {
            emit_line(to_stdout, format!("  RAM: {ram_gb:.1} GB"));
        }
        if let Some(active_toolchain) = &self.active_toolchain {
            emit_line(to_stdout, format!("  Toolchain: {active_toolchain}"));
        }
        if let Some(rust_version) = &self.rust_version {
            emit_line(to_stdout, format!("  Rust: {rust_version}"));
        }
        if let Some(cargo_version) = &self.cargo_version {
            emit_line(to_stdout, format!("  Cargo: {cargo_version}"));
        }
        if let Some(git_commit_sha) = &self.git_commit_sha {
            match &self.git_branch {
                Some(git_branch) => {
                    emit_line(to_stdout, format!("  Git: {git_branch} @ {git_commit_sha}"));
                }
                None => emit_line(to_stdout, format!("  Git: {git_commit_sha}")),
            }
        }
        emit_line(
            to_stdout,
            format!("  Build: {} (opt-level 3, LTO)", self.build_profile),
        );
    }
}

fn build_json_report(
    report: &BenchReport,
    total_elapsed: Duration,
    config: JsonRunConfig,
    environment: DetectedEnvironment,
) -> JsonBenchmarkReport {
    let summary = compute_report_summary(report);
    let sections = report
        .sections
        .iter()
        .map(|section| {
            let section_id = stable_slug(&section.title);
            let rows = section
                .rows
                .iter()
                .map(|row| JsonRow {
                    scenario_id: format!("{}__{}", section_id, stable_slug(&row.scenario)),
                    scenario: row.scenario.clone(),
                    category: categorize_scenario(&section.title, &row.scenario)
                        .id()
                        .to_owned(),
                    csqlite: row.csqlite.as_ref().map(JsonMeasurement::from_measurement),
                    fsqlite: row.fsqlite.as_ref().map(JsonMeasurement::from_measurement),
                    ratio_fsqlite_over_csqlite: row_ratio(row),
                })
                .collect();
            JsonSection {
                section_id,
                title: section.title.clone(),
                description: section.description.clone(),
                rows,
            }
        })
        .collect();

    JsonBenchmarkReport {
        schema_version: JSON_REPORT_SCHEMA_V3.to_owned(),
        generated_at_utc: chrono_stamp(),
        total_elapsed_ms: u64::try_from(total_elapsed.as_millis()).unwrap_or(u64::MAX),
        config,
        environment,
        ci_regression_gate: build_ci_regression_gate(report, &summary),
        summary,
        sections,
    }
}

#[allow(clippy::too_many_lines)]
fn benchmark_json_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://frankensqlite.dev/schemas/fsqlite-e2e/comprehensive-bench-report.v3.json",
        "title": "FrankenSQLite comprehensive benchmark JSON report",
        "type": "object",
        "additionalProperties": false,
        "required": [
            "schema_version",
            "generated_at_utc",
            "total_elapsed_ms",
            "config",
            "environment",
            "summary",
            "ci_regression_gate",
            "sections"
        ],
        "properties": {
            "schema_version": {
                "const": JSON_REPORT_SCHEMA_V3
            },
            "generated_at_utc": {
                "type": "string"
            },
            "total_elapsed_ms": {
                "type": "integer",
                "minimum": 0
            },
            "config": {
                "type": "object",
                "additionalProperties": true,
                "required": ["quick", "warmup_iterations", "min_iterations", "max_iterations", "target_duration_secs", "row_counts"]
            },
            "environment": {
                "type": "object",
                "additionalProperties": true,
                "required": ["arch", "build_profile"]
            },
            "summary": {
                "type": "object",
                "additionalProperties": false,
                "required": [
                    "total_scenarios",
                    "franken_faster",
                    "comparable",
                    "csqlite_faster",
                    "avg_ratio",
                    "average_ratio",
                    "geomean_ratio",
                    "median_ratio",
                    "p90_ratio",
                    "p99_ratio",
                    "primary_metric",
                    "per_category",
                    "per_category_weighted"
                ],
                "properties": {
                    "total_scenarios": {"type": "integer", "minimum": 0},
                    "franken_faster": {"type": "integer", "minimum": 0},
                    "comparable": {"type": "integer", "minimum": 0},
                    "csqlite_faster": {"type": "integer", "minimum": 0},
                    "avg_ratio": {
                        "type": ["number", "null"],
                        "description": "Continuity metric only; not the primary score."
                    },
                    "average_ratio": {
                        "type": ["number", "null"],
                        "description": "Backward-compatible alias for avg_ratio; not the primary score."
                    },
                    "geomean_ratio": {"type": ["number", "null"]},
                    "median_ratio": {"type": ["number", "null"]},
                    "p90_ratio": {"type": ["number", "null"]},
                    "p99_ratio": {"type": ["number", "null"]},
                    "primary_metric": {"const": "per_category_weighted.score"},
                    "per_category": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["read_single", "read_aggregate", "write_single", "write_bulk", "concurrent_writers", "mixed"],
                        "properties": {
                            "read_single": {"$ref": "#/$defs/category_stats"},
                            "read_aggregate": {"$ref": "#/$defs/category_stats"},
                            "write_single": {"$ref": "#/$defs/category_stats"},
                            "write_bulk": {"$ref": "#/$defs/category_stats"},
                            "concurrent_writers": {"$ref": "#/$defs/category_stats"},
                            "mixed": {"$ref": "#/$defs/category_stats"}
                        }
                    },
                    "per_category_weighted": {"$ref": "#/$defs/weighted_category_score"}
                }
            },
            "ci_regression_gate": {
                "type": "object",
                "additionalProperties": false,
                "required": ["schema_version", "bead_id", "depends_on_bead_id", "status", "thresholds", "observed"],
                "properties": {
                    "schema_version": {"const": CI_REGRESSION_GATE_SCHEMA_V2},
                    "bead_id": {"const": CI_REGRESSION_GATE_BEAD_ID},
                    "depends_on_bead_id": {"const": CI_REGRESSION_BASELINE_BEAD_ID},
                    "status": {"const": CI_REGRESSION_GATE_STATUS_RICH_SCORECARD},
                    "thresholds": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": [
                            "avg_ratio_baseline",
                            "avg_ratio_max",
                            "mt_p95_ratio_max",
                            "primary_score_max_regression_pct",
                            "geomean_max_regression_pct",
                            "per_category_geomean_max_regression_pct",
                            "p90_max_regression_pct",
                            "threshold_source"
                        ],
                        "properties": {
                            "avg_ratio_baseline": {"type": "number"},
                            "avg_ratio_max": {"type": ["number", "null"]},
                            "mt_p95_ratio_max": {"type": ["number", "null"]},
                            "primary_score_max_regression_pct": {"type": "number"},
                            "geomean_max_regression_pct": {"type": "number"},
                            "per_category_geomean_max_regression_pct": {"type": "number"},
                            "p90_max_regression_pct": {"type": "number"},
                            "threshold_source": {"type": "string"}
                        }
                    },
                    "observed": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": [
                            "avg_ratio",
                            "primary_score",
                            "geomean_ratio",
                            "median_ratio",
                            "p90_ratio",
                            "max_mt_p95_ratio",
                            "max_mt_p95_scenario_id"
                        ],
                        "properties": {
                            "avg_ratio": {"type": ["number", "null"]},
                            "primary_score": {"type": ["number", "null"]},
                            "geomean_ratio": {"type": ["number", "null"]},
                            "median_ratio": {"type": ["number", "null"]},
                            "p90_ratio": {"type": ["number", "null"]},
                            "max_mt_p95_ratio": {
                                "type": ["number", "null"],
                                "description": "Worst fsqlite/csqlite p95 latency ratio among multithreaded concurrent-writer rows."
                            },
                            "max_mt_p95_scenario_id": {"type": ["string", "null"]}
                        }
                    }
                }
            },
            "sections": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["section_id", "title", "description", "rows"],
                    "properties": {
                        "section_id": {"type": "string"},
                        "title": {"type": "string"},
                        "description": {"type": "string"},
                        "rows": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["scenario_id", "scenario", "category", "csqlite", "fsqlite", "ratio_fsqlite_over_csqlite"],
                                "properties": {
                                    "scenario_id": {"type": "string"},
                                    "scenario": {"type": "string"},
                                    "category": {"$ref": "#/$defs/scenario_category"},
                                    "csqlite": {"anyOf": [{"$ref": "#/$defs/measurement"}, {"type": "null"}]},
                                    "fsqlite": {"anyOf": [{"$ref": "#/$defs/measurement"}, {"type": "null"}]},
                                    "ratio_fsqlite_over_csqlite": {"type": ["number", "null"]}
                                }
                            }
                        }
                    }
                }
            }
        },
        "$defs": {
            "scenario_category": {
                "type": "string",
                "enum": ["read_single", "read_aggregate", "write_single", "write_bulk", "concurrent_writers", "mixed"]
            },
            "category_stats": {
                "type": "object",
                "additionalProperties": false,
                "required": ["n", "avg_ratio", "geomean_ratio", "median_ratio", "p90_ratio", "p99_ratio"],
                "properties": {
                    "n": {"type": "integer", "minimum": 0},
                    "avg_ratio": {"type": ["number", "null"]},
                    "geomean_ratio": {"type": ["number", "null"]},
                    "median_ratio": {"type": ["number", "null"]},
                    "p90_ratio": {"type": ["number", "null"]},
                    "p99_ratio": {"type": ["number", "null"]}
                }
            },
            "weighted_category_score": {
                "type": "object",
                "additionalProperties": false,
                "required": ["primary", "score", "weights", "observed_weight", "missing_categories"],
                "properties": {
                    "primary": {"const": true},
                    "score": {"type": ["number", "null"]},
                    "weights": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["read_single", "read_aggregate", "write_single", "write_bulk", "concurrent_writers", "mixed"],
                        "properties": {
                            "read_single": {"type": "number"},
                            "read_aggregate": {"type": "number"},
                            "write_single": {"type": "number"},
                            "write_bulk": {"type": "number"},
                            "concurrent_writers": {"type": "number"},
                            "mixed": {"type": "number"}
                        }
                    },
                    "observed_weight": {"type": "number", "minimum": 0},
                    "missing_categories": {
                        "type": "array",
                        "items": {"$ref": "#/$defs/scenario_category"}
                    }
                }
            },
            "measurement": {
                "type": "object",
                "additionalProperties": false,
                "required": ["median_ms", "mean_ms", "min_ms", "p95_ms", "p99_ms", "stddev_ms", "cv_pct", "rows_per_sec", "us_per_row", "iterations"],
                "properties": {
                    "median_ms": {"type": "number", "minimum": 0},
                    "mean_ms": {"type": "number", "minimum": 0},
                    "min_ms": {"type": "number", "minimum": 0},
                    "p95_ms": {"type": "number", "minimum": 0},
                    "p99_ms": {"type": "number", "minimum": 0},
                    "stddev_ms": {"type": "number", "minimum": 0},
                    "cv_pct": {"type": "number", "minimum": 0},
                    "rows_per_sec": {"type": "number", "minimum": 0},
                    "us_per_row": {"type": "number", "minimum": 0},
                    "iterations": {"type": "integer", "minimum": 1}
                }
            }
        }
    })
}

fn print_benchmark_json_schema() {
    match serde_json::to_string_pretty(&benchmark_json_schema()) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("ERROR: Could not serialize benchmark JSON schema: {err}");
            std::process::exit(1);
        }
    }
}

fn emit_line(to_stdout: bool, line: impl AsRef<str>) {
    if to_stdout {
        println!("{}", line.as_ref());
    } else {
        eprintln!("{}", line.as_ref());
    }
}

impl BenchReport {
    fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    fn add_section(&mut self, title: &str, description: &str) -> &mut ReportSection {
        self.sections.push(ReportSection {
            title: title.to_string(),
            description: description.to_string(),
            rows: Vec::new(),
        });
        self.sections.last_mut().unwrap()
    }

    fn print(&self, total_elapsed: Duration, environment: &DetectedEnvironment) {
        println!("\n{}", "=".repeat(140));
        println!("  COMPREHENSIVE BENCHMARK: FrankenSQLite vs C SQLite");
        println!("  {}", chrono_stamp());
        environment.print(true);
        println!(
            "  Total benchmark time: {:.1}s",
            total_elapsed.as_secs_f64()
        );
        println!("{}\n", "=".repeat(140));

        for section in &self.sections {
            println!("\n## {}", section.title);
            if !section.description.is_empty() {
                println!("   {}\n", section.description);
            }

            // Header
            println!(
                "  {:<42} {:>12} {:>12} {:>12} {:>12} {:>16} {:>8} {:>8}",
                "Scenario",
                "C SQLite",
                "FrankenSQLite",
                "C rows/s",
                "F rows/s",
                "Ratio",
                "CV%(C)",
                "CV%(F)"
            );
            println!("  {}", "-".repeat(136));

            for row in &section.rows {
                let cs_time = row
                    .csqlite
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), |m| format_duration(m.median()));
                let fs_time = row
                    .fsqlite
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), |m| format_duration(m.median()));
                let cs_rps = row
                    .csqlite
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), |m| format_rps(m.rows_per_sec()));
                let fs_rps = row
                    .fsqlite
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), |m| format_rps(m.rows_per_sec()));
                let cs_cv = row
                    .csqlite
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), |m| format!("{:.1}%", m.cv_percent()));
                let fs_cv = row
                    .fsqlite
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), |m| format!("{:.1}%", m.cv_percent()));

                let ratio = match (&row.csqlite, &row.fsqlite) {
                    (Some(c), Some(f)) => {
                        let r = f.median().as_nanos() as f64 / c.median().as_nanos() as f64;
                        if r < 1.0 {
                            format!("{:.2}x \x1b[32mfaster\x1b[0m", 1.0 / r)
                        } else if r > 1.0 {
                            format!("{:.2}x \x1b[31mslower\x1b[0m", r)
                        } else {
                            "1.00x equal".to_string()
                        }
                    }
                    _ => "N/A".to_string(),
                };

                println!(
                    "  {:<42} {:>12} {:>12} {:>12} {:>12} {:>16} {:>8} {:>8}",
                    row.scenario, cs_time, fs_time, cs_rps, fs_rps, ratio, cs_cv, fs_cv
                );
            }
        }

        // Summary statistics
        println!("\n{}", "=".repeat(120));
        println!("  SUMMARY STATISTICS");
        println!("{}\n", "=".repeat(120));

        let summary = compute_report_summary(self);
        if let Some(avg_ratio) = summary.average_ratio {
            println!(
                "  Total scenarios: {}  |  FrankenSQLite faster: {}  |  Comparable: {}  |  C SQLite faster: {}",
                summary.total_scenarios,
                summary.franken_faster,
                summary.comparable,
                summary.csqlite_faster
            );
            println!(
                "  Average time ratio (FrankenSQLite / C SQLite): {:.2}x",
                avg_ratio
            );
        }

        println!();
    }

    fn write_html(&self, path: &str) {
        let mut html = String::with_capacity(32 * 1024);

        // Collect JSON data for charts.
        let mut sections_json = String::from("[");
        for (si, section) in self.sections.iter().enumerate() {
            if si > 0 {
                sections_json.push(',');
            }
            sections_json.push_str(&format!(
                r#"{{"title":{},"description":{},"rows":["#,
                json_string(&section.title),
                json_string(&section.description),
            ));
            for (ri, row) in section.rows.iter().enumerate() {
                if ri > 0 {
                    sections_json.push(',');
                }
                let cs_median_ns = row
                    .csqlite
                    .as_ref()
                    .map_or(0.0, |m| m.median().as_nanos() as f64);
                let fs_median_ns = row
                    .fsqlite
                    .as_ref()
                    .map_or(0.0, |m| m.median().as_nanos() as f64);
                let cs_rps = row.csqlite.as_ref().map_or(0.0, Measurement::rows_per_sec);
                let fs_rps = row.fsqlite.as_ref().map_or(0.0, Measurement::rows_per_sec);
                let cs_mean_ns = row
                    .csqlite
                    .as_ref()
                    .map_or(0.0, |m| m.mean().as_nanos() as f64);
                let fs_mean_ns = row
                    .fsqlite
                    .as_ref()
                    .map_or(0.0, |m| m.mean().as_nanos() as f64);
                let cs_min_ns = row
                    .csqlite
                    .as_ref()
                    .map_or(0.0, |m| m.min().as_nanos() as f64);
                let fs_min_ns = row
                    .fsqlite
                    .as_ref()
                    .map_or(0.0, |m| m.min().as_nanos() as f64);
                let cs_stddev_ns = row
                    .csqlite
                    .as_ref()
                    .map_or(0.0, |m| m.stddev().as_nanos() as f64);
                let fs_stddev_ns = row
                    .fsqlite
                    .as_ref()
                    .map_or(0.0, |m| m.stddev().as_nanos() as f64);
                let cs_iters = row.csqlite.as_ref().map_or(0, |m| m.durations.len());
                let fs_iters = row.fsqlite.as_ref().map_or(0, |m| m.durations.len());
                let cs_cv = row.csqlite.as_ref().map_or(0.0, Measurement::cv_percent);
                let fs_cv = row.fsqlite.as_ref().map_or(0.0, Measurement::cv_percent);
                let cs_p95_ns = row
                    .csqlite
                    .as_ref()
                    .map_or(0.0, |m| m.p95().as_nanos() as f64);
                let fs_p95_ns = row
                    .fsqlite
                    .as_ref()
                    .map_or(0.0, |m| m.p95().as_nanos() as f64);
                let ratio = if cs_median_ns > 0.0 {
                    fs_median_ns / cs_median_ns
                } else {
                    0.0
                };
                sections_json.push_str(&format!(
                    r#"{{"scenario":{},"cs_median_ns":{cs_median_ns},"fs_median_ns":{fs_median_ns},"cs_rps":{cs_rps},"fs_rps":{fs_rps},"cs_mean_ns":{cs_mean_ns},"fs_mean_ns":{fs_mean_ns},"cs_min_ns":{cs_min_ns},"fs_min_ns":{fs_min_ns},"cs_stddev_ns":{cs_stddev_ns},"fs_stddev_ns":{fs_stddev_ns},"cs_iters":{cs_iters},"fs_iters":{fs_iters},"cs_cv":{cs_cv:.1},"fs_cv":{fs_cv:.1},"cs_p95_ns":{cs_p95_ns},"fs_p95_ns":{fs_p95_ns},"ratio":{ratio}}}"#,
                    json_string(&row.scenario),
                ));
            }
            sections_json.push_str("]}");
        }
        sections_json.push(']');

        // Summary stats.
        let summary = compute_report_summary(self);
        let avg_ratio = summary.average_ratio.unwrap_or(1.0);

        html.push_str(&format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>FrankenSQLite vs C SQLite — Benchmark Report</title>
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.7/dist/chart.umd.min.js"></script>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
<script>
tailwind.config = {{
  theme: {{
    extend: {{
      fontFamily: {{
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      }},
    }},
  }},
}}
</script>
<style>
  body {{ font-family: 'Inter', system-ui, sans-serif; }}
  .gradient-bg {{ background: linear-gradient(135deg, #0f172a 0%, #1e293b 50%, #0f172a 100%); }}
  .card {{ background: rgba(30, 41, 59, 0.8); backdrop-filter: blur(12px); border: 1px solid rgba(148, 163, 184, 0.1); }}
  .glow {{ box-shadow: 0 0 40px rgba(59, 130, 246, 0.15); }}
  .stat-card {{ transition: transform 0.2s, box-shadow 0.2s; }}
  .stat-card:hover {{ transform: translateY(-2px); box-shadow: 0 8px 30px rgba(0,0,0,0.3); }}
  .faster {{ color: #34d399; }}
  .slower {{ color: #f87171; }}
  .equal {{ color: #94a3b8; }}
  .bar-cs {{ background: linear-gradient(90deg, #3b82f6, #60a5fa); }}
  .bar-fs {{ background: linear-gradient(90deg, #f59e0b, #fbbf24); }}
  table th {{ position: sticky; top: 0; z-index: 10; }}
  .section-nav a {{ transition: all 0.15s; }}
  .section-nav a:hover {{ background: rgba(59, 130, 246, 0.2); }}
  .section-nav a.active {{ background: rgba(59, 130, 246, 0.3); border-left-color: #3b82f6; }}
</style>
</head>
<body class="gradient-bg min-h-screen text-slate-200">

<!-- Hero Header -->
<header class="py-12 px-6 text-center border-b border-slate-700/50">
  <div class="max-w-5xl mx-auto">
    <div class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-blue-500/10 border border-blue-500/20 text-blue-400 text-sm font-medium mb-6">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
      Performance Benchmark Report
    </div>
    <h1 class="text-4xl md:text-5xl font-extrabold bg-gradient-to-r from-white via-slate-200 to-slate-400 bg-clip-text text-transparent mb-4">
      FrankenSQLite vs C SQLite
    </h1>
    <p class="text-slate-400 text-lg max-w-2xl mx-auto">
      Comprehensive comparison across insertions, reads, concurrency, and mixed workloads.
      MVCC page-level versioning vs traditional WAL write lock.
    </p>
    <p class="text-slate-500 text-sm mt-4 font-mono">{}</p>
  </div>
</header>

<!-- Summary Cards -->
<section class="max-w-6xl mx-auto px-6 -mt-6">
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <div class="card rounded-xl p-5 stat-card glow">
      <div class="text-xs font-medium text-slate-400 uppercase tracking-wider mb-1">Total Scenarios</div>
      <div class="text-3xl font-bold text-white">{}</div>
    </div>
    <div class="card rounded-xl p-5 stat-card" style="box-shadow: 0 0 40px rgba(52,211,153,0.12);">
      <div class="text-xs font-medium text-slate-400 uppercase tracking-wider mb-1">FrankenSQLite Faster</div>
      <div class="text-3xl font-bold faster">{}</div>
    </div>
    <div class="card rounded-xl p-5 stat-card">
      <div class="text-xs font-medium text-slate-400 uppercase tracking-wider mb-1">Comparable</div>
      <div class="text-3xl font-bold equal">{}</div>
    </div>
    <div class="card rounded-xl p-5 stat-card" style="box-shadow: 0 0 40px rgba(248,113,113,0.10);">
      <div class="text-xs font-medium text-slate-400 uppercase tracking-wider mb-1">C SQLite Faster</div>
      <div class="text-3xl font-bold slower">{}</div>
    </div>
  </div>
  <div class="card rounded-xl p-5 mt-4 text-center">
    <span class="text-slate-400">Average time ratio (FrankenSQLite / C SQLite):</span>
    <span class="text-xl font-bold ml-2 {}">{avg_ratio:.2}x</span>
  </div>
</section>

<!-- Section Navigation -->
<nav class="section-nav max-w-6xl mx-auto px-6 mt-8">
  <div class="card rounded-xl p-4 flex flex-wrap gap-2" id="section-nav"></div>
</nav>

<!-- Benchmark Sections -->
<main class="max-w-6xl mx-auto px-6 py-8 space-y-10" id="sections-container"></main>

<!-- Footer -->
<footer class="border-t border-slate-700/50 py-8 text-center text-slate-500 text-sm">
  <p>Generated by <span class="text-slate-300 font-medium">comprehensive-bench</span> &mdash; FrankenSQLite E2E Benchmark Suite</p>
  <p class="mt-1">Clean-room Rust reimplementation of SQLite with MVCC page-level versioning</p>
</footer>

<script>
const DATA = {sections_json};

function fmtDuration(ns) {{
  if (ns === 0) return 'N/A';
  if (ns < 1e3) return ns.toFixed(0) + ' ns';
  if (ns < 1e6) return (ns / 1e3).toFixed(1) + ' \u00b5s';
  if (ns < 1e9) return (ns / 1e6).toFixed(2) + ' ms';
  return (ns / 1e9).toFixed(3) + ' s';
}}

function fmtRps(rps) {{
  if (rps === 0) return 'N/A';
  if (rps >= 1e6) return (rps / 1e6).toFixed(2) + 'M/s';
  if (rps >= 1e3) return (rps / 1e3).toFixed(1) + 'K/s';
  return rps.toFixed(0) + '/s';
}}

function ratioClass(r) {{
  if (r < 0.95) return 'faster';
  if (r > 1.05) return 'slower';
  return 'equal';
}}

function ratioText(r) {{
  if (r === 0) return 'N/A';
  if (r < 1.0) return (1/r).toFixed(2) + 'x faster';
  if (r > 1.0) return r.toFixed(2) + 'x slower';
  return '1.00x equal';
}}

// Build section navigation
const nav = document.getElementById('section-nav');
DATA.forEach((sec, i) => {{
  const a = document.createElement('a');
  a.href = '#section-' + i;
  a.textContent = sec.title.replace(/\u2014/g, '-').substring(0, 40) + (sec.title.length > 40 ? '...' : '');
  a.className = 'block px-3 py-1.5 rounded-lg text-sm text-slate-300 border-l-2 border-transparent hover:text-white cursor-pointer';
  nav.appendChild(a);
}});

// Build sections
const container = document.getElementById('sections-container');
DATA.forEach((sec, si) => {{
  const div = document.createElement('div');
  div.id = 'section-' + si;
  div.className = 'scroll-mt-24';

  // Only create chart for sections with paired data
  const hasChart = sec.rows.some(r => r.cs_median_ns > 0 && r.fs_median_ns > 0);
  const chartId = 'chart-' + si;

  let tableRows = '';
  sec.rows.forEach(r => {{
    const rc = ratioClass(r.ratio);
    const csCV = r.cs_cv !== undefined ? r.cs_cv.toFixed(1) + '%' : 'N/A';
    const fsCV = r.fs_cv !== undefined ? r.fs_cv.toFixed(1) + '%' : 'N/A';
    const csP95 = r.cs_p95_ns ? fmtDuration(r.cs_p95_ns) : 'N/A';
    const fsP95 = r.fs_p95_ns ? fmtDuration(r.fs_p95_ns) : 'N/A';
    tableRows += `<tr class="border-b border-slate-700/30 hover:bg-slate-700/20 transition-colors">
      <td class="py-3 px-4 text-sm font-medium text-slate-200">${{r.scenario}}</td>
      <td class="py-3 px-4 text-sm font-mono text-right text-blue-400" title="p95: ${{csP95}}">${{fmtDuration(r.cs_median_ns)}}</td>
      <td class="py-3 px-4 text-sm font-mono text-right text-amber-400" title="p95: ${{fsP95}}">${{fmtDuration(r.fs_median_ns)}}</td>
      <td class="py-3 px-4 text-sm font-mono text-right text-blue-300">${{fmtRps(r.cs_rps)}}</td>
      <td class="py-3 px-4 text-sm font-mono text-right text-amber-300">${{fmtRps(r.fs_rps)}}</td>
      <td class="py-3 px-4 text-sm font-mono text-right font-semibold ${{rc}}">${{ratioText(r.ratio)}}</td>
      <td class="py-3 px-4 text-sm font-mono text-right text-slate-500">${{csCV}} / ${{fsCV}}</td>
    </tr>`;
  }});

  div.innerHTML = `
    <div class="card rounded-2xl overflow-hidden glow">
      <div class="px-6 py-5 border-b border-slate-700/50">
        <h2 class="text-xl font-bold text-white">${{sec.title}}</h2>
        ${{sec.description ? '<p class="text-sm text-slate-400 mt-1">' + sec.description + '</p>' : ''}}
      </div>
      ${{hasChart ? '<div class="px-6 py-4 border-b border-slate-700/30"><canvas id="' + chartId + '" height="' + Math.max(60, sec.rows.length * 28) + '"></canvas></div>' : ''}}
      <div class="overflow-x-auto">
        <table class="w-full text-left">
          <thead>
            <tr class="bg-slate-800/80 text-xs font-semibold text-slate-400 uppercase tracking-wider">
              <th class="py-3 px-4">Scenario</th>
              <th class="py-3 px-4 text-right">C SQLite</th>
              <th class="py-3 px-4 text-right">FrankenSQLite</th>
              <th class="py-3 px-4 text-right">C rows/s</th>
              <th class="py-3 px-4 text-right">F rows/s</th>
              <th class="py-3 px-4 text-right">Ratio</th>
              <th class="py-3 px-4 text-right" title="Coefficient of Variation">CV% (C/F)</th>
            </tr>
          </thead>
          <tbody>${{tableRows}}</tbody>
        </table>
      </div>
    </div>`;

  container.appendChild(div);

  // Create horizontal bar chart
  if (hasChart) {{
    const ctx = document.getElementById(chartId).getContext('2d');
    const labels = sec.rows.filter(r => r.cs_median_ns > 0 && r.fs_median_ns > 0).map(r => {{
      const s = r.scenario;
      return s.length > 50 ? s.substring(0, 47) + '...' : s;
    }});
    const csData = sec.rows.filter(r => r.cs_median_ns > 0 && r.fs_median_ns > 0).map(r => r.cs_median_ns / 1e6);
    const fsData = sec.rows.filter(r => r.cs_median_ns > 0 && r.fs_median_ns > 0).map(r => r.fs_median_ns / 1e6);

    new Chart(ctx, {{
      type: 'bar',
      data: {{
        labels: labels,
        datasets: [
          {{
            label: 'C SQLite (ms)',
            data: csData,
            backgroundColor: 'rgba(59, 130, 246, 0.7)',
            borderColor: 'rgba(96, 165, 250, 1)',
            borderWidth: 1,
            borderRadius: 4,
          }},
          {{
            label: 'FrankenSQLite (ms)',
            data: fsData,
            backgroundColor: 'rgba(245, 158, 11, 0.7)',
            borderColor: 'rgba(251, 191, 36, 1)',
            borderWidth: 1,
            borderRadius: 4,
          }},
        ],
      }},
      options: {{
        indexAxis: 'y',
        responsive: true,
        maintainAspectRatio: false,
        plugins: {{
          legend: {{
            labels: {{ color: '#94a3b8', font: {{ family: 'Inter', size: 12 }} }},
          }},
          tooltip: {{
            callbacks: {{
              label: function(ctx) {{
                return ctx.dataset.label + ': ' + ctx.parsed.x.toFixed(2) + ' ms';
              }}
            }}
          }},
        }},
        scales: {{
          x: {{
            type: 'logarithmic',
            title: {{ display: true, text: 'Time (ms, log scale)', color: '#64748b' }},
            ticks: {{ color: '#64748b', font: {{ family: 'JetBrains Mono', size: 11 }} }},
            grid: {{ color: 'rgba(71, 85, 105, 0.3)' }},
          }},
          y: {{
            ticks: {{ color: '#94a3b8', font: {{ family: 'Inter', size: 11 }} }},
            grid: {{ display: false }},
          }},
        }},
      }},
    }});
  }}
}});

// Intersection observer for nav highlighting
const observer = new IntersectionObserver((entries) => {{
  entries.forEach(entry => {{
    if (entry.isIntersecting) {{
      const idx = entry.target.id.replace('section-', '');
      document.querySelectorAll('.section-nav a').forEach((a, i) => {{
        a.classList.toggle('active', i === parseInt(idx));
      }});
    }}
  }});
}}, {{ threshold: 0.3 }});
document.querySelectorAll('[id^="section-"]').forEach(el => observer.observe(el));
</script>
</body>
</html>"#,
            chrono_stamp(),
            if avg_ratio < 1.0 { "faster" } else { "slower" },
            summary.total_scenarios,
            summary.franken_faster,
            summary.comparable,
            summary.csqlite_faster,
        ));

        let Ok(mut file) = std::fs::File::create(path) else {
            eprintln!("ERROR: Could not create HTML file at {path}");
            return;
        };
        if file.write_all(html.as_bytes()).is_ok() {
            eprintln!("HTML report written to: {path}");
        } else {
            eprintln!("ERROR: Failed to write HTML file");
        }
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl ReportSection {
    fn add_row(
        &mut self,
        scenario: &str,
        csqlite: Option<Measurement>,
        fsqlite: Option<Measurement>,
    ) {
        self.rows.push(ReportRow {
            scenario: scenario.to_string(),
            csqlite,
            fsqlite,
        });
    }
}

fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{nanos} ns")
    } else if nanos < 1_000_000 {
        format!("{:.1} us", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2} ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.3} s", nanos as f64 / 1_000_000_000.0)
    }
}

fn format_rps(rps: f64) -> String {
    if rps >= 1_000_000.0 {
        format!("{:.2}M/s", rps / 1_000_000.0)
    } else if rps >= 1_000.0 {
        format!("{:.1}K/s", rps / 1_000.0)
    } else {
        format!("{:.0}/s", rps)
    }
}

fn chrono_stamp() -> String {
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert unix timestamp to readable date.
    let days = now / 86400;
    let secs_in_day = now % 86400;
    let hours = secs_in_day / 3600;
    let mins = (secs_in_day % 3600) / 60;
    let secs = secs_in_day % 60;
    // Approximate year/month/day from days since epoch.
    let (year, month, day) = days_to_ymd(days);
    format!("{year}-{month:02}-{day:02} {hours:02}:{mins:02}:{secs:02} UTC")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Simplified Gregorian calendar conversion.
    let mut y = 1970;
    let mut remaining = days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for &md in month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    (y, m + 1, remaining + 1)
}

fn timestamp_filename(base: &str, ext: &str) -> String {
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d) = days_to_ymd(now / 86400);
    let h = (now % 86400) / 3600;
    let min = (now % 3600) / 60;
    format!("{base}_{y}{m:02}{d:02}_{h:02}{min:02}.{ext}")
}

fn print_usage() {
    eprintln!(
        "Usage:
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --quick
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --filter insert
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --json-out report.json --no-html
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --json-stdout --no-html
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- --print-json-schema

Flags:
  --quick              Run the reduced benchmark matrix.
  --filter <text>      Run only sections whose names match <text>.
  --html <path>        Write the HTML report to an explicit path.
  --no-html            Skip HTML report generation.
  --json               Write the JSON report to a timestamped file.
  --json-out <path>    Write the JSON report to an explicit path.
  --json-stdout        Emit only the structured JSON report to stdout.
  --print-json-schema  Emit the standardized benchmark JSON schema and exit.
  --help, -h           Show this help text."
    );
}

fn parse_cli_args(args: &[String]) -> Result<CliOptions, String> {
    let mut options = CliOptions {
        quick: false,
        filter: None,
        html_path: None,
        emit_html: true,
        emit_timestamped_json: false,
        json_out_path: None,
        json_stdout: false,
        print_json_schema: false,
    };

    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--quick" => {
                options.quick = true;
                index += 1;
            }
            "--filter" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "expected a value after --filter".to_owned())?;
                options.filter = Some(value.clone());
                index += 2;
            }
            "--html" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "expected a path after --html".to_owned())?;
                options.html_path = Some(value.clone());
                options.emit_html = true;
                index += 2;
            }
            "--no-html" => {
                options.emit_html = false;
                index += 1;
            }
            "--json" => {
                options.emit_timestamped_json = true;
                index += 1;
            }
            "--json-out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "expected a path after --json-out".to_owned())?;
                options.json_out_path = Some(value.clone());
                index += 2;
            }
            "--json-stdout" => {
                options.json_stdout = true;
                index += 1;
            }
            "--print-json-schema" => {
                options.print_json_schema = true;
                index += 1;
            }
            unknown => {
                return Err(format!("unrecognized argument `{unknown}`"));
            }
        }
    }

    Ok(options)
}

fn print_run_banner(
    to_stdout: bool,
    options: &CliOptions,
    row_counts: &[usize],
    environment: &DetectedEnvironment,
) {
    emit_line(to_stdout, format!("\n{}", "=".repeat(80)));
    emit_line(
        to_stdout,
        "  Comprehensive FrankenSQLite vs C SQLite Benchmark",
    );
    emit_line(to_stdout, "=".repeat(80));
    environment.print(to_stdout);
    emit_line(
        to_stdout,
        format!("  Mode: {}", if options.quick { "quick" } else { "full" }),
    );
    emit_line(
        to_stdout,
        format!("  Row counts: {:?}", row_counts.iter().collect::<Vec<_>>()),
    );
    emit_line(
        to_stdout,
        format!(
            "  Measurement: {WARMUP_ITERS} warmup, {MIN_ITERS}-{MAX_ITERS} iters, target {:.0}s",
            TARGET_DURATION.as_secs_f64()
        ),
    );
    if let Some(filter) = &options.filter {
        emit_line(to_stdout, format!("  Filter: {filter}"));
    }
    emit_line(to_stdout, "=".repeat(80));
    emit_line(to_stdout, "");
}

// ─── Section 1: Insert throughput by row count ─────────────────────────

fn bench_insert_by_row_count(
    report: &mut BenchReport,
    row_counts: &[usize],
    record_size: RecordSize,
) {
    let section = report.add_section(
        &format!(
            "INSERTThroughput — Single Transaction — {}",
            record_size.name()
        ),
        &format!(
            "Record: {}. All rows inserted in a single BEGIN..COMMIT.",
            record_size.description()
        ),
    );

    for &count in row_counts {
        eprint!(
            "  Benchmarking single-txn insert {count} rows ({})... ",
            record_size.name()
        );

        let csqlite_m = {
            let insert_sql = record_size.insert_sql_csqlite();
            let create_sql = record_size.create_table_sql();
            measure(&format!("csqlite_{count}"), count, || {
                let conn = rusqlite::Connection::open_in_memory().unwrap();
                apply_pragmas_csqlite(&conn);
                conn.execute_batch(&format!("{create_sql};")).unwrap();
                conn.execute_batch("BEGIN").unwrap();
                let mut stmt = conn.prepare(insert_sql).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    stmt.execute(rusqlite::params![i]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();
            })
        };

        let fsqlite_m = {
            let create_sql = record_size.create_table_sql();
            measure(&format!("fsqlite_{count}"), count, || {
                let conn = fsqlite::Connection::open(":memory:").unwrap();
                apply_pragmas_fsqlite(&conn);
                fs_execute(&conn, create_sql);
                fs_execute(&conn, "BEGIN");
                #[allow(clippy::cast_possible_wrap)]
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                for i in 0..count as i64 {
                    fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                }
                fs_execute(&conn, "COMMIT");
            })
        };

        eprintln!(
            "C={} F={}",
            format_duration(csqlite_m.median()),
            format_duration(fsqlite_m.median()),
        );

        section.add_row(&format!("{count} rows"), Some(csqlite_m), Some(fsqlite_m));
    }
}

// ─── Section 2: Insert throughput by transaction strategy ──────────────

fn bench_insert_by_txn_strategy(report: &mut BenchReport, row_counts: &[usize]) {
    let section = report.add_section(
        "INSERTThroughput — Transaction Strategy Comparison (small_3col)",
        "Compares autocommit, batched (1K/txn), and single-txn strategies.",
    );

    let record_size = RecordSize::Small;

    for &count in row_counts {
        // Skip autocommit for large counts (too slow).
        let do_autocommit = count <= 10_000;
        let batch_size = 1000.min(count);

        // --- Autocommit ---
        if do_autocommit {
            eprint!("  Benchmarking autocommit {count} rows... ");

            let cs = {
                let insert_sql = record_size.insert_sql_csqlite();
                let create_sql = record_size.create_table_sql();
                measure(&format!("cs_auto_{count}"), count, || {
                    let conn = rusqlite::Connection::open_in_memory().unwrap();
                    apply_pragmas_csqlite(&conn);
                    conn.execute_batch(&format!("{create_sql};")).unwrap();
                    let mut stmt = conn.prepare(insert_sql).unwrap();
                    #[allow(clippy::cast_possible_wrap)]
                    for i in 0..count as i64 {
                        stmt.execute(rusqlite::params![i]).unwrap();
                    }
                })
            };

            let fs = {
                let create_sql = record_size.create_table_sql();
                measure(&format!("fs_auto_{count}"), count, || {
                    let conn = fsqlite::Connection::open(":memory:").unwrap();
                    apply_pragmas_fsqlite(&conn);
                    fs_execute(&conn, create_sql);
                    let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                    #[allow(clippy::cast_possible_wrap)]
                    for i in 0..count as i64 {
                        fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                    }
                })
            };

            eprintln!(
                "C={} F={}",
                format_duration(cs.median()),
                format_duration(fs.median())
            );
            section.add_row(&format!("{count} rows / autocommit"), Some(cs), Some(fs));
        }

        // --- Batched ---
        eprint!("  Benchmarking batched {count} rows ({batch_size}/txn)... ");

        let cs = {
            let insert_sql = record_size.insert_sql_csqlite();
            let create_sql = record_size.create_table_sql();
            measure(&format!("cs_batch_{count}"), count, || {
                let conn = rusqlite::Connection::open_in_memory().unwrap();
                apply_pragmas_csqlite(&conn);
                conn.execute_batch(&format!("{create_sql};")).unwrap();
                let mut stmt = conn.prepare(insert_sql).unwrap();
                let num_batches = count.div_ceil(batch_size);
                #[allow(clippy::cast_possible_wrap)]
                for batch in 0..num_batches {
                    conn.execute_batch("BEGIN").unwrap();
                    let start = (batch * batch_size) as i64;
                    let end = ((batch + 1) * batch_size).min(count) as i64;
                    for i in start..end {
                        stmt.execute(rusqlite::params![i]).unwrap();
                    }
                    conn.execute_batch("COMMIT").unwrap();
                }
            })
        };

        let fs = {
            let create_sql = record_size.create_table_sql();
            measure(&format!("fs_batch_{count}"), count, || {
                let conn = fsqlite::Connection::open(":memory:").unwrap();
                apply_pragmas_fsqlite(&conn);
                fs_execute(&conn, create_sql);
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                let num_batches = count.div_ceil(batch_size);
                #[allow(clippy::cast_possible_wrap)]
                for batch in 0..num_batches {
                    fs_execute(&conn, "BEGIN");
                    let start = (batch * batch_size) as i64;
                    let end = ((batch + 1) * batch_size).min(count) as i64;
                    for i in start..end {
                        fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                    }
                    fs_execute(&conn, "COMMIT");
                }
            })
        };

        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / batched ({batch_size}/txn)"),
            Some(cs),
            Some(fs),
        );

        // --- Single txn ---
        eprint!("  Benchmarking single-txn {count} rows... ");

        let cs = {
            let insert_sql = record_size.insert_sql_csqlite();
            let create_sql = record_size.create_table_sql();
            measure(&format!("cs_txn_{count}"), count, || {
                let conn = rusqlite::Connection::open_in_memory().unwrap();
                apply_pragmas_csqlite(&conn);
                conn.execute_batch(&format!("{create_sql};")).unwrap();
                conn.execute_batch("BEGIN").unwrap();
                let mut stmt = conn.prepare(insert_sql).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    stmt.execute(rusqlite::params![i]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();
            })
        };

        let fs = {
            let create_sql = record_size.create_table_sql();
            measure(&format!("fs_txn_{count}"), count, || {
                let conn = fsqlite::Connection::open(":memory:").unwrap();
                apply_pragmas_fsqlite(&conn);
                fs_execute(&conn, create_sql);
                fs_execute(&conn, "BEGIN");
                #[allow(clippy::cast_possible_wrap)]
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                for i in 0..count as i64 {
                    fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                }
                fs_execute(&conn, "COMMIT");
            })
        };

        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} rows / single txn"), Some(cs), Some(fs));
    }
}

// ─── Section 3: Insert throughput by record size ───────────────────────

fn bench_insert_by_record_size(report: &mut BenchReport) {
    let section = report.add_section(
        "INSERTThroughput — Record Size Comparison (10K rows, single txn)",
        "Fixed 10K rows in a single transaction, varying payload size.",
    );

    let count = 10_000_usize;

    for &record_size in RecordSize::ALL {
        eprint!(
            "  Benchmarking 10K rows record size {}... ",
            record_size.name()
        );

        let cs = {
            let insert_sql = record_size.insert_sql_csqlite();
            let create_sql = record_size.create_table_sql();
            measure(&format!("cs_{}", record_size.name()), count, || {
                let conn = rusqlite::Connection::open_in_memory().unwrap();
                apply_pragmas_csqlite(&conn);
                conn.execute_batch(&format!("{create_sql};")).unwrap();
                conn.execute_batch("BEGIN").unwrap();
                let mut stmt = conn.prepare(insert_sql).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    stmt.execute(rusqlite::params![i]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();
            })
        };

        let fs = {
            let create_sql = record_size.create_table_sql();
            measure(&format!("fs_{}", record_size.name()), count, || {
                let conn = fsqlite::Connection::open(":memory:").unwrap();
                apply_pragmas_fsqlite(&conn);
                fs_execute(&conn, create_sql);
                fs_execute(&conn, "BEGIN");
                #[allow(clippy::cast_possible_wrap)]
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                for i in 0..count as i64 {
                    fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                }
                fs_execute(&conn, "COMMIT");
            })
        };

        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{} — {}", record_size.name(), record_size.description()),
            Some(cs),
            Some(fs),
        );
    }
}

// ─── Section 4: Concurrent writers ─────────────────────────────────────

fn bench_concurrent_writers(report: &mut BenchReport) {
    let section = report.add_section(
        CONCURRENT_WRITERS_SECTION_TITLE,
        &format!(
            "Each writer inserts {} rows into non-overlapping key ranges on the same \
             file-backed WAL database. Both engines spawn N OS threads each owning its \
             own connection; C SQLite uses WAL + busy_timeout, FrankenSQLite uses the \
             MVCC page-lock table via `PRAGMA fsqlite.concurrent_mode=ON` + \
             `BEGIN CONCURRENT` (falling back to plain BEGIN if the pragma is declined). \
             This mirrors the standalone `mt_mvcc_bench` harness.",
            CONCURRENT_ROWS_PER_THREAD
        ),
    );

    for &n_threads in CONCURRENT_THREAD_COUNTS {
        let total_rows = n_threads * CONCURRENT_ROWS_PER_THREAD;
        eprint!("  Benchmarking {n_threads} concurrent writers ({total_rows} total rows)... ");

        // C SQLite: file-backed WAL with multiple connections.
        let cs = measure(&format!("cs_concurrent_{n_threads}t"), total_rows, || {
            let runtime = RuntimeBuilder::new()
                .blocking_threads(n_threads, n_threads)
                .build()
                .expect("comprehensive benchmark runtime should build");
            let tmp = tempfile::NamedTempFile::new().unwrap();
            let path = tmp.path().to_str().unwrap().to_owned();
            {
                let setup = rusqlite::Connection::open(&path).unwrap();
                setup
                    .execute_batch(
                        "PRAGMA page_size = 4096;\
                         PRAGMA journal_mode = WAL;\
                         PRAGMA synchronous = NORMAL;\
                         PRAGMA cache_size = -64000;\
                         CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, score INTEGER);",
                    )
                    .unwrap();
            }

            let barrier = Arc::new(Barrier::new(n_threads));
            let handles: Vec<_> = (0..n_threads)
                .map(|tid| {
                    let p = path.clone();
                    let bar = Arc::clone(&barrier);
                    spawn_bench_task(&runtime, move || {
                        // Enter the start gate before any fallible setup so one
                        // worker error cannot strand the rest at the barrier.
                        bar.wait();
                        let conn = rusqlite::Connection::open(&p).unwrap();
                        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
                            .unwrap();

                        conn.execute_batch("BEGIN").unwrap();
                        #[allow(clippy::cast_possible_wrap)]
                        let base = tid as i64 * CONCURRENT_RANGE_SIZE;
                        let mut stmt = conn
                            .prepare("INSERT INTO bench VALUES (?1, ('t' || ?1), (?1 * 7))")
                            .unwrap();
                        #[allow(clippy::cast_possible_wrap)]
                        for i in 0..CONCURRENT_ROWS_PER_THREAD as i64 {
                            stmt.execute(rusqlite::params![base + i]).unwrap();
                        }
                        conn.execute_batch("COMMIT").unwrap();
                    })
                })
                .collect();

            for h in handles {
                h.wait();
            }
        });

        // FrankenSQLite: file-backed WAL with multiple connections, one per
        // OS thread — mirrors the C SQLite arm exactly. Connection is
        // !Send + !Sync so each thread must call fsqlite::Connection::open
        // locally inside its spawn closure. Uses BEGIN CONCURRENT where the
        // `fsqlite.concurrent_mode` pragma is accepted (the MVCC mode that's
        // the whole point of FrankenSQLite); falls back to plain BEGIN
        // otherwise. See also crates/fsqlite-e2e/src/bin/mt_mvcc_bench.rs
        // for the standalone reference implementation of this pattern.
        let fs = measure(&format!("fs_concurrent_{n_threads}t"), total_rows, || {
            let runtime = RuntimeBuilder::new()
                .blocking_threads(n_threads, n_threads)
                .build()
                .expect("comprehensive benchmark runtime should build");
            let tmp = tempfile::NamedTempFile::new().unwrap();
            let path = tmp.path().to_str().unwrap().to_owned();
            {
                let setup = fsqlite::Connection::open(&path).unwrap();
                apply_pragmas_fsqlite(&setup);
                fs_execute(
                    &setup,
                    "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, score INTEGER)",
                );
            }

            let barrier = Arc::new(Barrier::new(n_threads));
            let handles: Vec<_> = (0..n_threads)
                .map(|tid| {
                    let p = path.clone();
                    let bar = Arc::clone(&barrier);
                    spawn_bench_task(&runtime, move || {
                        // Enter the start gate before any fallible setup so one
                        // worker error cannot strand the rest at the barrier.
                        bar.wait();
                        let conn = fsqlite::Connection::open(&p).unwrap();
                        apply_pragmas_fsqlite(&conn);
                        let concurrent_ok =
                            conn.execute("PRAGMA fsqlite.concurrent_mode=ON;").is_ok();
                        let _ = conn.execute("PRAGMA busy_timeout=5000;");

                        let begin_sql = if concurrent_ok {
                            "BEGIN CONCURRENT"
                        } else {
                            "BEGIN"
                        };

                        // Mirror `mt_mvcc_bench`'s pattern: wrap the entire
                        // BEGIN + N*INSERT + COMMIT in a retry loop, because
                        // a BusySnapshot on any individual statement aborts
                        // the whole MVCC transaction (rollback required
                        // before re-BEGIN). Per-statement retries cannot
                        // recover once the containing txn is poisoned.
                        #[allow(clippy::cast_possible_wrap)]
                        let base = tid as i64 * CONCURRENT_RANGE_SIZE;
                        let mut retry_count = 0_u32;
                        const TXN_MAX_RETRIES: u32 = 64;
                        const TXN_BACKOFF_MS: u64 = 5;
                        'txn: loop {
                            if let Err(e) = conn.execute(begin_sql) {
                                if e.is_transient() && retry_count < TXN_MAX_RETRIES {
                                    retry_count += 1;
                                    std::thread::sleep(std::time::Duration::from_millis(
                                        TXN_BACKOFF_MS,
                                    ));
                                    continue;
                                }
                                panic!("fsqlite BEGIN failed after retries: {e}");
                            }
                            let stmt = conn
                                .prepare("INSERT INTO bench VALUES (?1, ('t' || ?1), (?1 * 7))")
                                .unwrap();
                            #[allow(clippy::cast_possible_wrap)]
                            for i in 0..CONCURRENT_ROWS_PER_THREAD as i64 {
                                match stmt
                                    .execute_with_params(&[fsqlite::SqliteValue::Integer(base + i)])
                                {
                                    Ok(_) => {}
                                    Err(e) if e.is_transient() && retry_count < TXN_MAX_RETRIES => {
                                        let _ = conn.execute("ROLLBACK");
                                        retry_count += 1;
                                        std::thread::sleep(std::time::Duration::from_millis(
                                            TXN_BACKOFF_MS,
                                        ));
                                        continue 'txn;
                                    }
                                    Err(e) => panic!(
                                        "fsqlite INSERT tid={tid} i={i} failed: {e} \
                                         (retry_count={retry_count})"
                                    ),
                                }
                            }
                            match conn.execute("COMMIT") {
                                Ok(_) => break 'txn,
                                Err(e) if e.is_transient() && retry_count < TXN_MAX_RETRIES => {
                                    let _ = conn.execute("ROLLBACK");
                                    retry_count += 1;
                                    std::thread::sleep(std::time::Duration::from_millis(
                                        TXN_BACKOFF_MS,
                                    ));
                                }
                                Err(e) => panic!(
                                    "fsqlite COMMIT tid={tid} failed: {e} \
                                     (retry_count={retry_count})"
                                ),
                            }
                        }
                    })
                })
                .collect();

            for h in handles {
                h.wait();
            }
        });

        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{n_threads} writers x {CONCURRENT_ROWS_PER_THREAD} rows"),
            Some(cs),
            Some(fs),
        );
    }

    // Also benchmark C SQLite single-threaded for the same total work (baseline).
    let section = report.add_section(
        "Concurrent Writers — C SQLite Single-Thread Baseline",
        "Same total row count as concurrent tests, but single-threaded file-backed C SQLite.",
    );

    for &n_threads in CONCURRENT_THREAD_COUNTS {
        let total_rows = n_threads * CONCURRENT_ROWS_PER_THREAD;
        eprint!("  Benchmarking C SQLite single-thread baseline ({total_rows} rows)... ");

        let cs_single = measure(&format!("cs_single_{n_threads}t_equiv"), total_rows, || {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            let path = tmp.path().to_str().unwrap().to_owned();
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "PRAGMA page_size = 4096;\
                     PRAGMA journal_mode = WAL;\
                     PRAGMA synchronous = NORMAL;\
                     PRAGMA cache_size = -64000;\
                     CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, score INTEGER);",
            )
            .unwrap();

            conn.execute_batch("BEGIN").unwrap();
            let mut stmt = conn
                .prepare("INSERT INTO bench VALUES (?1, ('t' || ?1), (?1 * 7))")
                .unwrap();
            #[allow(clippy::cast_possible_wrap)]
            for i in 0..total_rows as i64 {
                stmt.execute(rusqlite::params![i]).unwrap();
            }
            conn.execute_batch("COMMIT").unwrap();
        });

        eprintln!("C_single={}", format_duration(cs_single.median()));
        section.add_row(
            &format!("C SQLite 1 thread / {total_rows} rows (baseline)"),
            Some(cs_single),
            None,
        );
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn sample_measurement(label: &str, row_count: usize, durations_ms: &[u64]) -> Measurement {
        Measurement {
            label: label.to_owned(),
            durations: durations_ms
                .iter()
                .map(|ms| Duration::from_millis(*ms))
                .collect(),
            row_count,
        }
    }

    fn sample_report() -> BenchReport {
        let mut report = BenchReport::new();
        let section = report.add_section(
            "Insert Throughput",
            "Sequential insert benchmarking for parser-stable JSON output.",
        );
        section.add_row(
            "100 rows / small record",
            Some(sample_measurement("csqlite", 100, &[1, 1, 2])),
            Some(sample_measurement("frankensqlite", 100, &[2, 2, 3])),
        );
        report
    }

    #[test]
    fn benchmark_pragmas_disable_time_travel_capture() {
        assert!(
            FSQLITE_BENCHMARK_PRAGMAS.iter().any(|pragma| pragma
                .eq_ignore_ascii_case("PRAGMA fsqlite_capture_time_travel_snapshots=false;")),
            "comprehensive-bench should profile benchmark workloads, not optional time-travel snapshot cloning"
        );
    }

    #[test]
    fn spawn_bench_task_runs_on_runtime_blocking_pool() {
        let runtime = RuntimeBuilder::new()
            .blocking_threads(1, 1)
            .build()
            .expect("benchmark runtime should build for test");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_for_task = Arc::clone(&counter);

        let handle = spawn_bench_task(&runtime, move || {
            counter_for_task.fetch_add(1, Ordering::Relaxed);
        });

        handle.wait();
        assert_eq!(
            counter.load(Ordering::Relaxed),
            1,
            "benchmark task should run exactly once",
        );
    }

    #[test]
    fn spawn_bench_task_propagates_panics() {
        let runtime = RuntimeBuilder::new()
            .blocking_threads(1, 1)
            .build()
            .expect("benchmark runtime should build for test");
        let handle = spawn_bench_task(&runtime, || -> () {
            panic!("benchmark worker panic should surface");
        });

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle.wait()))
            .expect_err("wait should propagate worker panic");
        let message = panic_payload_to_string(panic);
        assert!(
            message.contains("benchmark worker panic should surface"),
            "panic payload should mention original worker failure: {message}",
        );
    }

    #[test]
    fn parse_cli_args_supports_machine_readable_flags() {
        let args = vec![
            "comprehensive-bench".to_owned(),
            "--quick".to_owned(),
            "--filter".to_owned(),
            "insert".to_owned(),
            "--json-out".to_owned(),
            "bench.json".to_owned(),
            "--json-stdout".to_owned(),
            "--no-html".to_owned(),
        ];

        let options = parse_cli_args(&args).expect("cli args should parse");

        assert_eq!(
            options,
            CliOptions {
                quick: true,
                filter: Some("insert".to_owned()),
                html_path: None,
                emit_html: false,
                emit_timestamped_json: false,
                json_out_path: Some("bench.json".to_owned()),
                json_stdout: true,
                print_json_schema: false,
            }
        );
    }

    #[test]
    fn parse_cli_args_rejects_missing_filter_value() {
        let args = vec!["comprehensive-bench".to_owned(), "--filter".to_owned()];

        let error = parse_cli_args(&args).expect_err("missing filter value should error");
        assert!(
            error.contains("expected a value after --filter"),
            "unexpected error: {error}",
        );
    }

    #[test]
    fn scenario_categories_use_canonical_ids() {
        assert_eq!(ScenarioCategory::MixedOltp.id(), "mixed");
        assert_eq!(
            categorize_scenario("Mixed OLTP Workload at Scale", "5K ops (80r/20w)").id(),
            "mixed"
        );
        assert_eq!(
            categorize_scenario(
                "INSERTThroughput — Transaction Strategy Comparison (small_3col)",
                "100 rows / autocommit",
            )
            .id(),
            "write_single"
        );
        assert_eq!(
            categorize_scenario(
                "INSERTThroughput — Transaction Strategy Comparison (small_3col)",
                "1000 rows / batched (1000/txn)",
            )
            .id(),
            "write_bulk"
        );
        assert_eq!(
            categorize_scenario("Read-After-Write Query Performance", "100 rows / COUNT(*)").id(),
            "read_aggregate"
        );
        assert_eq!(
            categorize_scenario(
                "Read-After-Write Query Performance",
                "100 rows / point lookup (PK)",
            )
            .id(),
            "read_single"
        );
        assert_eq!(
            categorize_scenario(
                "JOIN Performance — Multi-Table Queries",
                "100 orders / INNER JOIN"
            )
            .id(),
            "read_single"
        );
        assert_eq!(
            categorize_scenario(
                "JOIN Performance — Multi-Table Queries",
                "100 orders / JOIN + GROUP BY",
            )
            .id(),
            "read_aggregate"
        );
    }

    #[test]
    fn weighted_category_score_uses_ratio_geomean_rollup() {
        let mut per_category = BTreeMap::new();
        for category in ScenarioCategory::ALL {
            per_category.insert(category.id().to_owned(), ratio_stats(&[]));
        }
        per_category.insert(
            "read_single".to_owned(),
            JsonCategoryRatioStats {
                n: 1,
                avg_ratio: Some(4.0),
                geomean_ratio: Some(4.0),
                median_ratio: Some(4.0),
                p90_ratio: Some(4.0),
                p99_ratio: Some(4.0),
            },
        );
        per_category.insert(
            "write_single".to_owned(),
            JsonCategoryRatioStats {
                n: 1,
                avg_ratio: Some(1.0),
                geomean_ratio: Some(1.0),
                median_ratio: Some(1.0),
                p90_ratio: Some(1.0),
                p99_ratio: Some(1.0),
            },
        );

        let score = weighted_category_score(&per_category);
        let expected = ((4.0_f64.ln() * 0.35) / 0.65).exp();

        assert!(
            (score.score.expect("score should exist") - expected).abs() < 1.0e-12,
            "weighted score should be a weighted geometric ratio rollup",
        );
        assert!((score.observed_weight - 0.65).abs() < 1.0e-12);
        assert!(score.missing_categories.contains(&"mixed".to_owned()));
    }

    #[test]
    fn build_json_report_uses_stable_ids_and_summary() {
        let report = sample_report();
        let json = build_json_report(
            &report,
            Duration::from_secs(2),
            JsonRunConfig {
                quick: true,
                filter: Some("insert".to_owned()),
                warmup_iterations: WARMUP_ITERS,
                min_iterations: MIN_ITERS,
                max_iterations: MAX_ITERS,
                target_duration_secs: TARGET_DURATION.as_secs(),
                row_counts: vec![100],
                html_output_path: Some("report.html".to_owned()),
                json_output_path: Some("report.json".to_owned()),
                json_stdout: false,
            },
            DetectedEnvironment {
                os: Some("TestOS".to_owned()),
                arch: "x86_64".to_owned(),
                kernel_release: Some("6.0.0-test".to_owned()),
                cpu_model: Some("Test CPU".to_owned()),
                cpu_cores: Some(8),
                ram_gb: Some(32.0),
                active_toolchain: Some("nightly-x86_64-unknown-linux-gnu".to_owned()),
                rust_version: Some("rustc test".to_owned()),
                cargo_version: Some("cargo test".to_owned()),
                git_commit_sha: Some("0123456789abcdef".to_owned()),
                git_branch: Some("main".to_owned()),
                build_profile: "release-perf".to_owned(),
            },
        );

        assert_eq!(json.schema_version, JSON_REPORT_SCHEMA_V3);
        assert_eq!(json.summary.total_scenarios, 1);
        assert_eq!(json.summary.primary_metric, "per_category_weighted.score");
        assert_eq!(json.summary.per_category["write_bulk"].n, 1);
        assert!(
            (json
                .summary
                .per_category_weighted
                .score
                .expect("primary score should exist")
                - json
                    .summary
                    .geomean_ratio
                    .expect("geomean ratio should exist"))
            .abs()
                < 1.0e-12
        );
        assert_eq!(
            json.ci_regression_gate.schema_version,
            CI_REGRESSION_GATE_SCHEMA_V2
        );
        assert_eq!(json.ci_regression_gate.bead_id, CI_REGRESSION_GATE_BEAD_ID);
        assert_eq!(
            json.ci_regression_gate.depends_on_bead_id,
            CI_REGRESSION_BASELINE_BEAD_ID
        );
        assert_eq!(
            json.ci_regression_gate.thresholds.avg_ratio_baseline,
            CI_REGRESSION_BASELINE_AVG_RATIO
        );
        assert_eq!(json.sections.len(), 1);
        assert_eq!(json.sections[0].section_id, "insert-throughput");
        assert_eq!(
            json.sections[0].rows[0].scenario_id,
            "insert-throughput__100-rows-small-record",
        );
        assert_eq!(json.sections[0].rows[0].category, "write_bulk");
        assert!(
            json.summary
                .average_ratio
                .expect("average ratio should exist for comparable row")
                > 1.0
        );
    }

    #[test]
    fn ci_regression_gate_tracks_multithread_p95_ratio() {
        let mut report = BenchReport::new();
        let section = report.add_section(CONCURRENT_WRITERS_SECTION_TITLE, "test");
        section.add_row(
            "2 writers x 1000 rows",
            Some(sample_measurement("csqlite", 2_000, &[10, 10, 10])),
            Some(sample_measurement("frankensqlite", 2_000, &[20, 25, 30])),
        );
        section.add_row(
            "8 writers x 1000 rows",
            Some(sample_measurement("csqlite", 8_000, &[10, 10, 10])),
            Some(sample_measurement("frankensqlite", 8_000, &[15, 15, 15])),
        );

        let json = build_json_report(
            &report,
            Duration::from_secs(1),
            JsonRunConfig {
                quick: true,
                filter: Some("concurrent".to_owned()),
                warmup_iterations: WARMUP_ITERS,
                min_iterations: MIN_ITERS,
                max_iterations: MAX_ITERS,
                target_duration_secs: TARGET_DURATION.as_secs(),
                row_counts: vec![100],
                html_output_path: None,
                json_output_path: Some("report.json".to_owned()),
                json_stdout: false,
            },
            DetectedEnvironment {
                os: None,
                arch: "x86_64".to_owned(),
                kernel_release: None,
                cpu_model: None,
                cpu_cores: Some(8),
                ram_gb: None,
                active_toolchain: None,
                rust_version: None,
                cargo_version: None,
                git_commit_sha: None,
                git_branch: Some("main".to_owned()),
                build_profile: "release-perf".to_owned(),
            },
        );

        assert_eq!(
            json.ci_regression_gate.observed.max_mt_p95_scenario_id,
            Some(
                "concurrent-writers-c-sqlite-wal-vs-frankensqlite-mvcc__2-writers-x-1000-rows"
                    .to_owned(),
            )
        );
        assert_eq!(json.ci_regression_gate.observed.max_mt_p95_ratio, Some(3.0));
        assert_eq!(
            json.ci_regression_gate.observed.primary_score,
            json.summary.per_category_weighted.score
        );
        assert_eq!(json.sections[0].rows[0].category, "concurrent_writers");
    }

    #[test]
    fn benchmark_json_schema_exposes_gate_metrics() {
        let schema = benchmark_json_schema();

        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            JSON_REPORT_SCHEMA_V3
        );
        assert_eq!(
            schema["properties"]["ci_regression_gate"]["properties"]["bead_id"]["const"],
            CI_REGRESSION_GATE_BEAD_ID
        );
        assert_eq!(
            schema["properties"]["summary"]["properties"]["primary_metric"]["const"],
            "per_category_weighted.score"
        );
        assert_eq!(
            schema["properties"]["ci_regression_gate"]["properties"]["thresholds"]["properties"]["primary_score_max_regression_pct"]
                ["type"],
            "number"
        );
        assert_eq!(
            schema["properties"]["sections"]["items"]["properties"]["rows"]["items"]["properties"]
                ["category"]["$ref"],
            "#/$defs/scenario_category"
        );
        assert_eq!(schema["$defs"]["scenario_category"]["enum"][5], "mixed");
        assert_eq!(
            schema["properties"]["summary"]["properties"]["per_category"]["required"][5],
            "mixed"
        );
        assert_eq!(
            schema["properties"]["ci_regression_gate"]["properties"]["observed"]["properties"]["primary_score"]
                ["type"][0],
            "number"
        );
    }
}

// ─── Section 5: Read-after-write performance ───────────────────────────

fn bench_read_after_write(report: &mut BenchReport, row_counts: &[usize]) {
    let section = report.add_section(
        "Read-After-Write Query Performance",
        "Insert N rows, then benchmark various SELECT patterns. Record: small_3col.",
    );

    let record_size = RecordSize::Small;

    for &count in row_counts {
        // Skip very large for query benchmarks.
        if count > 100_000 {
            continue;
        }

        eprint!("  Setting up {count} rows for read benchmarks... ");

        // Set up C SQLite.
        let cs_conn = {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            apply_pragmas_csqlite(&conn);
            let create_sql = record_size.create_table_sql();
            conn.execute_batch(&format!("{create_sql};")).unwrap();
            conn.execute_batch("BEGIN").unwrap();
            {
                let mut stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    stmt.execute(rusqlite::params![i]).unwrap();
                }
            }
            conn.execute_batch("COMMIT").unwrap();
            // Create secondary index.
            conn.execute_batch("CREATE INDEX idx_name ON bench(name);")
                .unwrap();
            conn
        };

        // Set up FrankenSQLite.
        let fs_conn = {
            let conn = fsqlite::Connection::open(":memory:").unwrap();
            apply_pragmas_fsqlite(&conn);
            fs_execute(&conn, record_size.create_table_sql());
            fs_execute(&conn, "BEGIN");
            {
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                }
            }
            fs_execute(&conn, "COMMIT");
            fs_execute(&conn, "CREATE INDEX idx_name ON bench(name)");
            conn
        };

        eprintln!("done.");

        // Full table scan.
        eprint!("    Full table scan... ");
        let cs = {
            let mut stmt = cs_conn.prepare("SELECT * FROM bench").unwrap();
            measure(&format!("cs_scan_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn.prepare("SELECT * FROM bench").unwrap();
        let fs = measure(&format!("fs_scan_{count}"), count, || {
            let _rows = fs_stmt.query().unwrap();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / full table scan"),
            Some(cs),
            Some(fs),
        );

        // Point lookup by PK.
        eprint!("    Point lookup (PK)... ");
        let target_id = (count / 2) as i64;
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT * FROM bench WHERE id = ?1")
                .unwrap();
            measure(&format!("cs_pk_{count}"), 1, || {
                let _rows = collect_rusqlite_rows(&mut stmt, rusqlite::params![target_id]).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT * FROM bench WHERE id = ?1")
            .unwrap();
        let fs = measure(&format!("fs_pk_{count}"), 1, || {
            let _row = fs_stmt
                .query_row_with_params(&[fsqlite::SqliteValue::Integer(target_id)])
                .unwrap();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / point lookup (PK)"),
            Some(cs),
            Some(fs),
        );

        // Range scan (10% of table).
        let range_size = count / 10;
        let range_start = (count / 4) as i64;
        #[allow(clippy::cast_possible_wrap)]
        let range_end = range_start + range_size as i64;
        eprint!("    Range scan ({range_size} rows)... ");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT * FROM bench WHERE id >= ?1 AND id < ?2")
                .unwrap();
            measure(&format!("cs_range_{count}"), range_size, || {
                let _rows =
                    collect_rusqlite_rows(&mut stmt, rusqlite::params![range_start, range_end])
                        .unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT * FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        let fs = measure(&format!("fs_range_{count}"), range_size, || {
            let _rows = fs_stmt
                .query_with_params(&[
                    fsqlite::SqliteValue::Integer(range_start),
                    fsqlite::SqliteValue::Integer(range_end),
                ])
                .unwrap();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / range scan ({range_size} rows)"),
            Some(cs),
            Some(fs),
        );

        // COUNT(*)
        eprint!("    COUNT(*)... ");
        let cs = {
            let mut stmt = cs_conn.prepare("SELECT COUNT(*) FROM bench").unwrap();
            measure(&format!("cs_count_{count}"), 1, || {
                let _: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            })
        };
        let fs_stmt = fs_conn.prepare("SELECT COUNT(*) FROM bench").unwrap();
        let fs = measure(&format!("fs_count_{count}"), 1, || {
            let _row = fs_stmt.query_row().unwrap();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} rows / COUNT(*)"), Some(cs), Some(fs));

        // Aggregate SUM + GROUP BY.
        eprint!("    SUM + GROUP BY... ");
        let cs = {
            // Group by integer division to get ~10 groups.
            #[allow(clippy::cast_possible_wrap)]
            let group_divisor = (count / 10).max(1) as i64;
            let sql = format!(
                "SELECT (id / {group_divisor}), SUM(value) FROM bench GROUP BY (id / {group_divisor})"
            );
            let mut stmt = cs_conn.prepare(&sql).unwrap();
            measure(&format!("cs_groupby_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs = {
            #[allow(clippy::cast_possible_wrap)]
            let group_divisor = (count / 10).max(1) as i64;
            let sql = format!(
                "SELECT (id / {group_divisor}), SUM(value) FROM bench GROUP BY (id / {group_divisor})"
            );
            let stmt = fs_conn.prepare(&sql).unwrap();
            measure(&format!("fs_groupby_{count}"), count, || {
                let _rows = stmt.query().unwrap();
            })
        };
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / SUM + GROUP BY (~10 groups)"),
            Some(cs),
            Some(fs),
        );

        // Indexed lookup on secondary index.
        eprint!("    Indexed lookup (secondary)... ");
        let target_name = format!("user_{target_id}");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT * FROM bench WHERE name = ?1")
                .unwrap();
            measure(&format!("cs_idx_{count}"), 1, || {
                let _rows =
                    collect_rusqlite_rows(&mut stmt, rusqlite::params![target_name.clone()])
                        .unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT * FROM bench WHERE name = ?1")
            .unwrap();
        let target_name_param = [fsqlite::SqliteValue::Text(target_name.into())];
        let fs = measure(&format!("fs_idx_{count}"), 1, || {
            let _rows = fs_stmt.query_with_params(&target_name_param).unwrap();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / indexed lookup (secondary)"),
            Some(cs),
            Some(fs),
        );

        // ORDER BY + LIMIT.
        eprint!("    ORDER BY + LIMIT 20... ");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT * FROM bench ORDER BY value DESC LIMIT 20")
                .unwrap();
            measure(&format!("cs_order_{count}"), 20, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT * FROM bench ORDER BY value DESC LIMIT 20")
            .unwrap();
        let fs = measure(&format!("fs_order_{count}"), 20, || {
            let _rows = fs_stmt.query().unwrap();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / ORDER BY + LIMIT 20"),
            Some(cs),
            Some(fs),
        );
    }
}

// ─── Section 6: Update and delete throughput ───────────────────────────

fn bench_update_delete(report: &mut BenchReport, row_counts: &[usize]) {
    let section = report.add_section(
        "UPDATE/DELETEThroughput",
        "Pre-populated table with N rows. Measures batch update (10% of rows) and batch delete (5% of rows).",
    );

    let record_size = RecordSize::Small;

    for &count in row_counts {
        if count > 100_000 {
            continue;
        }

        let update_count = count / 10;
        let delete_count = count / 20;

        // Batch update: update 10% of rows.
        eprint!("  Benchmarking update {update_count}/{count} rows... ");

        let cs = {
            let insert_sql = record_size.insert_sql_csqlite();
            let create_sql = record_size.create_table_sql();
            measure(&format!("cs_update_{count}"), update_count, || {
                let conn = rusqlite::Connection::open_in_memory().unwrap();
                apply_pragmas_csqlite(&conn);
                conn.execute_batch(&format!("{create_sql};")).unwrap();
                conn.execute_batch("BEGIN").unwrap();
                let mut ins = conn.prepare(insert_sql).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    ins.execute(rusqlite::params![i]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();

                conn.execute_batch("BEGIN").unwrap();
                let mut upd = conn
                    .prepare("UPDATE bench SET value = ?2 WHERE id = ?1")
                    .unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..update_count as i64 {
                    let id = i * 10; // Every 10th row.
                    upd.execute(rusqlite::params![id, 999.99]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();
            })
        };

        let fs = {
            let create_sql = record_size.create_table_sql();
            measure(&format!("fs_update_{count}"), update_count, || {
                let conn = fsqlite::Connection::open(":memory:").unwrap();
                apply_pragmas_fsqlite(&conn);
                fs_execute(&conn, create_sql);
                fs_execute(&conn, "BEGIN");
                #[allow(clippy::cast_possible_wrap)]
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                for i in 0..count as i64 {
                    fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                }
                fs_execute(&conn, "COMMIT");

                fs_execute(&conn, "BEGIN");
                let update = conn
                    .prepare("UPDATE bench SET value = ?2 WHERE id = ?1")
                    .unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..update_count as i64 {
                    let id = i * 10;
                    fs_stmt_execute_with_params(
                        &update,
                        &[
                            fsqlite::SqliteValue::Integer(id),
                            fsqlite::SqliteValue::Float(999.99),
                        ],
                    );
                }
                fs_execute(&conn, "COMMIT");
            })
        };

        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / update {update_count} rows"),
            Some(cs),
            Some(fs),
        );

        // Batch delete: delete 5% of rows.
        eprint!("  Benchmarking delete {delete_count}/{count} rows... ");

        let cs = {
            let insert_sql = record_size.insert_sql_csqlite();
            let create_sql = record_size.create_table_sql();
            measure(&format!("cs_delete_{count}"), delete_count, || {
                let conn = rusqlite::Connection::open_in_memory().unwrap();
                apply_pragmas_csqlite(&conn);
                conn.execute_batch(&format!("{create_sql};")).unwrap();
                conn.execute_batch("BEGIN").unwrap();
                let mut ins = conn.prepare(insert_sql).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..count as i64 {
                    ins.execute(rusqlite::params![i]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();

                conn.execute_batch("BEGIN").unwrap();
                let mut del = conn.prepare("DELETE FROM bench WHERE id = ?1").unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..delete_count as i64 {
                    let id = i * 20; // Every 20th row.
                    del.execute(rusqlite::params![id]).unwrap();
                }
                conn.execute_batch("COMMIT").unwrap();
            })
        };

        let fs = {
            let create_sql = record_size.create_table_sql();
            measure(&format!("fs_delete_{count}"), delete_count, || {
                let conn = fsqlite::Connection::open(":memory:").unwrap();
                apply_pragmas_fsqlite(&conn);
                fs_execute(&conn, create_sql);
                fs_execute(&conn, "BEGIN");
                #[allow(clippy::cast_possible_wrap)]
                let stmt = conn.prepare(record_size.insert_sql_csqlite()).unwrap();
                for i in 0..count as i64 {
                    fs_stmt_execute_with_params(&stmt, &[fsqlite::SqliteValue::Integer(i)]);
                }
                fs_execute(&conn, "COMMIT");

                fs_execute(&conn, "BEGIN");
                let delete = conn.prepare("DELETE FROM bench WHERE id = ?1").unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 0..delete_count as i64 {
                    let id = i * 20;
                    fs_stmt_execute_with_params(&delete, &[fsqlite::SqliteValue::Integer(id)]);
                }
                fs_execute(&conn, "COMMIT");
            })
        };

        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / delete {delete_count} rows"),
            Some(cs),
            Some(fs),
        );
    }
}

// ─── Section 7: Mixed OLTP workload at scale ───────────────────────────

struct Rng64 {
    state: u64,
}

impl Rng64 {
    const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    #[allow(clippy::cast_possible_truncation)]
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % (bound as u64)) as usize
    }
}

fn bench_mixed_oltp(report: &mut BenchReport) {
    let section = report.add_section(
        "Mixed OLTP Workload (80% read / 20% write)",
        "Pre-seeded with 5K rows. Runs 5K operations with realistic distribution: \
         40% point lookups, 20% range scans, 20% aggregates, 15% inserts, 3% updates, 2% deletes.",
    );

    let ops = 5_000_usize;
    let seed_rows = 5_000_usize;

    eprint!("  Benchmarking mixed OLTP C SQLite... ");

    let cs = measure("cs_oltp", ops, || {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        apply_pragmas_csqlite(&conn);
        conn.execute_batch(
            "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, score INTEGER);",
        )
        .unwrap();
        conn.execute_batch("BEGIN").unwrap();
        {
            let mut stmt = conn
                .prepare("INSERT INTO bench VALUES (?1, ('name_' || ?1), (?1 * 7))")
                .unwrap();
            #[allow(clippy::cast_possible_wrap)]
            for i in 1..=seed_rows as i64 {
                stmt.execute(rusqlite::params![i]).unwrap();
            }
        }
        conn.execute_batch("COMMIT").unwrap();

        let mut rng = Rng64::new(42);
        #[allow(clippy::cast_possible_wrap)]
        let mut next_id = seed_rows as i64 + 1;
        let mut select_pt = conn.prepare("SELECT * FROM bench WHERE id = ?1").unwrap();
        let mut select_range = conn
            .prepare("SELECT COUNT(*) FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        let mut select_agg = conn
            .prepare("SELECT COUNT(*), SUM(score) FROM bench")
            .unwrap();
        let mut insert = conn
            .prepare("INSERT INTO bench VALUES (?1, ('name_' || ?1), (?1 * 7))")
            .unwrap();
        let mut update = conn
            .prepare("UPDATE bench SET score = ?2 WHERE id = ?1")
            .unwrap();
        let mut delete = conn.prepare("DELETE FROM bench WHERE id = ?1").unwrap();

        #[allow(clippy::cast_possible_wrap)]
        for _ in 0..ops {
            let roll = rng.next_usize(100);
            if roll < 40 {
                let id = (rng.next_usize(seed_rows) + 1) as i64;
                let _ = collect_rusqlite_rows(&mut select_pt, rusqlite::params![id]);
            } else if roll < 60 {
                let start = (rng.next_usize(seed_rows.saturating_sub(50)) + 1) as i64;
                let _: i64 = select_range
                    .query_row(rusqlite::params![start, start + 50], |r| r.get(0))
                    .unwrap_or(0);
            } else if roll < 80 {
                let _: (i64, i64) = select_agg
                    .query_row([], |r| Ok((r.get(0).unwrap(), r.get(1).unwrap())))
                    .unwrap();
            } else if roll < 95 {
                let _ = insert.execute(rusqlite::params![next_id]);
                next_id += 1;
            } else if roll < 98 {
                let id = (rng.next_usize(seed_rows) + 1) as i64;
                let _ = update.execute(rusqlite::params![id, id * 99]);
            } else {
                let id = (rng.next_usize(seed_rows) + 1) as i64;
                let _ = delete.execute(rusqlite::params![id]);
            }
        }
    });

    eprintln!("C={}", format_duration(cs.median()));

    eprint!("  Benchmarking mixed OLTP FrankenSQLite... ");

    let fs = measure("fs_oltp", ops, || {
        let conn = fsqlite::Connection::open(":memory:").unwrap();
        apply_pragmas_fsqlite(&conn);
        fs_execute(
            &conn,
            "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT, score INTEGER)",
        );
        let seed_insert = conn
            .prepare("INSERT INTO bench VALUES (?1, ('name_' || ?1), (?1 * 7))")
            .unwrap();
        fs_execute(&conn, "BEGIN");
        #[allow(clippy::cast_possible_wrap)]
        for i in 1..=seed_rows as i64 {
            fs_stmt_execute_with_params(&seed_insert, &[fsqlite::SqliteValue::Integer(i)]);
        }
        fs_execute(&conn, "COMMIT");

        let mut rng = Rng64::new(42);
        #[allow(clippy::cast_possible_wrap)]
        let mut next_id = seed_rows as i64 + 1;
        let select_pt = conn.prepare("SELECT * FROM bench WHERE id = ?1").unwrap();
        let select_range = conn
            .prepare("SELECT COUNT(*) FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        let select_agg = conn
            .prepare("SELECT COUNT(*), SUM(score) FROM bench")
            .unwrap();
        let insert = conn
            .prepare("INSERT INTO bench VALUES (?1, ('name_' || ?1), (?1 * 7))")
            .unwrap();
        let update = conn
            .prepare("UPDATE bench SET score = ?2 WHERE id = ?1")
            .unwrap();
        let delete = conn.prepare("DELETE FROM bench WHERE id = ?1").unwrap();

        #[allow(clippy::cast_possible_wrap)]
        for _ in 0..ops {
            let roll = rng.next_usize(100);
            if roll < 40 {
                let id = (rng.next_usize(seed_rows) + 1) as i64;
                let _ = select_pt.query_row_with_params(&[fsqlite::SqliteValue::Integer(id)]);
            } else if roll < 60 {
                let start = (rng.next_usize(seed_rows.saturating_sub(50)) + 1) as i64;
                let _ = select_range.query_row_with_params(&[
                    fsqlite::SqliteValue::Integer(start),
                    fsqlite::SqliteValue::Integer(start + 50),
                ]);
            } else if roll < 80 {
                let _ = select_agg.query_row();
            } else if roll < 95 {
                let _ = insert.execute_with_params(&[fsqlite::SqliteValue::Integer(next_id)]);
                next_id += 1;
            } else if roll < 98 {
                let id = (rng.next_usize(seed_rows) + 1) as i64;
                let _ = update.execute_with_params(&[
                    fsqlite::SqliteValue::Integer(id),
                    fsqlite::SqliteValue::Integer(id * 99),
                ]);
            } else {
                let id = (rng.next_usize(seed_rows) + 1) as i64;
                let _ = delete.execute_with_params(&[fsqlite::SqliteValue::Integer(id)]);
            }
        }
    });

    eprintln!("F={}", format_duration(fs.median()));

    section.add_row("5K ops (80r/20w) on 5K-row table", Some(cs), Some(fs));
}

// ─── Section 8: JOIN performance ────────────────────────────────────────

fn bench_join_performance(report: &mut BenchReport, row_counts: &[usize]) {
    let section = report.add_section(
        "JOIN Performance — Multi-Table Queries",
        "Two related tables (orders+customers). Measures INNER JOIN, LEFT JOIN, self-join, and JOIN with aggregation.",
    );

    for &count in row_counts {
        if count > 100_000 {
            continue;
        }

        let customer_count = count / 10; // 10x fewer customers than orders.
        let customer_count = customer_count.max(10);

        eprint!("  Setting up JOIN tables ({count} orders, {customer_count} customers)... ");

        // C SQLite setup + benchmarks.
        let cs_conn = {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            apply_pragmas_csqlite(&conn);
            conn.execute_batch(
                "CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT, region TEXT);\
                 CREATE TABLE orders (id INTEGER PRIMARY KEY, customer_id INTEGER, amount REAL, status TEXT);",
            ).unwrap();
            conn.execute_batch("BEGIN").unwrap();
            {
                let mut cstmt = conn.prepare("INSERT INTO customers VALUES (?1, ('cust_' || ?1), CASE ?1 % 4 WHEN 0 THEN 'North' WHEN 1 THEN 'South' WHEN 2 THEN 'East' ELSE 'West' END)").unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 1..=customer_count as i64 {
                    cstmt.execute(rusqlite::params![i]).unwrap();
                }
                let mut ostmt = conn.prepare("INSERT INTO orders VALUES (?1, ((?1 % ?2) + 1), (?1 * 9.99 / 100.0), CASE ?1 % 3 WHEN 0 THEN 'pending' WHEN 1 THEN 'shipped' ELSE 'delivered' END)").unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 1..=count as i64 {
                    ostmt
                        .execute(rusqlite::params![i, customer_count as i64])
                        .unwrap();
                }
            }
            conn.execute_batch("COMMIT").unwrap();
            conn.execute_batch("CREATE INDEX idx_orders_cust ON orders(customer_id);")
                .unwrap();
            conn
        };

        let fs_conn = {
            let conn = fsqlite::Connection::open(":memory:").unwrap();
            apply_pragmas_fsqlite(&conn);
            fs_execute(
                &conn,
                "CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT, region TEXT)",
            );
            fs_execute(
                &conn,
                "CREATE TABLE orders (id INTEGER PRIMARY KEY, customer_id INTEGER, amount REAL, status TEXT)",
            );
            fs_execute(&conn, "BEGIN");
            #[allow(clippy::cast_possible_wrap)]
            for i in 1..=customer_count as i64 {
                let region = match i % 4 {
                    0 => "North",
                    1 => "South",
                    2 => "East",
                    _ => "West",
                };
                fs_execute(
                    &conn,
                    &format!("INSERT INTO customers VALUES ({i}, 'cust_{i}', '{region}')"),
                );
            }
            #[allow(clippy::cast_possible_wrap)]
            for i in 1..=count as i64 {
                let cid = (i % customer_count as i64) + 1;
                let amount = i as f64 * 9.99 / 100.0;
                let status = match i % 3 {
                    0 => "pending",
                    1 => "shipped",
                    _ => "delivered",
                };
                fs_execute(
                    &conn,
                    &format!("INSERT INTO orders VALUES ({i}, {cid}, {amount}, '{status}')"),
                );
            }
            fs_execute(&conn, "COMMIT");
            fs_execute(&conn, "CREATE INDEX idx_orders_cust ON orders(customer_id)");
            conn
        };

        eprintln!("done.");

        // INNER JOIN.
        eprint!("    INNER JOIN... ");
        let cs = {
            let mut stmt = cs_conn.prepare("SELECT c.name, o.amount FROM customers c INNER JOIN orders o ON o.customer_id = c.id").unwrap();
            measure(&format!("cs_inner_join_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT c.name, o.amount FROM customers c INNER JOIN orders o ON o.customer_id = c.id")
            .unwrap();
        let fs = measure(&format!("fs_inner_join_{count}"), count, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} orders / INNER JOIN"), Some(cs), Some(fs));

        // LEFT JOIN.
        eprint!("    LEFT JOIN... ");
        let cs = {
            let mut stmt = cs_conn.prepare("SELECT c.name, o.amount FROM customers c LEFT JOIN orders o ON o.customer_id = c.id").unwrap();
            measure(&format!("cs_left_join_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT c.name, o.amount FROM customers c LEFT JOIN orders o ON o.customer_id = c.id")
            .unwrap();
        let fs = measure(&format!("fs_left_join_{count}"), count, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} orders / LEFT JOIN"), Some(cs), Some(fs));

        // JOIN + GROUP BY aggregate.
        eprint!("    JOIN + GROUP BY aggregate... ");
        let cs = {
            let mut stmt = cs_conn.prepare("SELECT c.name, COUNT(*), SUM(o.amount) FROM customers c JOIN orders o ON o.customer_id = c.id GROUP BY c.name").unwrap();
            measure(&format!("cs_join_agg_{count}"), customer_count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT c.name, COUNT(*), SUM(o.amount) FROM customers c JOIN orders o ON o.customer_id = c.id GROUP BY c.name")
            .unwrap();
        let fs = measure(&format!("fs_join_agg_{count}"), customer_count, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} orders / JOIN + GROUP BY"),
            Some(cs),
            Some(fs),
        );

        // JOIN + GROUP BY + HAVING.
        eprint!("    JOIN + GROUP BY + HAVING... ");
        let threshold = count as f64 * 0.05; // Customers with > 5% of orders.
        let cs = {
            let sql = format!(
                "SELECT c.name, COUNT(*) cnt FROM customers c JOIN orders o ON o.customer_id = c.id GROUP BY c.name HAVING cnt > {threshold}"
            );
            let mut stmt = cs_conn.prepare(&sql).unwrap();
            measure(&format!("cs_join_having_{count}"), customer_count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs = {
            let sql = format!(
                "SELECT c.name, COUNT(*) cnt FROM customers c JOIN orders o ON o.customer_id = c.id GROUP BY c.name HAVING cnt > {threshold}"
            );
            let stmt = fs_conn.prepare(&sql).unwrap();
            measure(&format!("fs_join_having_{count}"), customer_count, || {
                let _ = stmt.query();
            })
        };
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} orders / JOIN + HAVING"),
            Some(cs),
            Some(fs),
        );
    }
}

// ─── Section 9: Subquery & CTE performance ──────────────────────────────

fn bench_subquery_cte(report: &mut BenchReport, row_counts: &[usize]) {
    let section = report.add_section(
        "Subquery & CTE Performance",
        "Measures scalar subqueries, EXISTS, IN subqueries, and recursive CTEs.",
    );

    for &count in row_counts {
        if count > 100_000 {
            continue;
        }

        eprint!("  Setting up subquery tables ({count} rows)... ");

        let cs_conn = {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            apply_pragmas_csqlite(&conn);
            conn.execute_batch(
                "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL, category_id INTEGER);\
                 CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT);",
            ).unwrap();
            conn.execute_batch("BEGIN").unwrap();
            {
                let cat_count = (count / 20).max(5);
                let mut cstmt = conn
                    .prepare("INSERT INTO categories VALUES (?1, ('cat_' || ?1))")
                    .unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 1..=cat_count as i64 {
                    cstmt.execute(rusqlite::params![i]).unwrap();
                }
                let mut pstmt = conn.prepare("INSERT INTO products VALUES (?1, ('prod_' || ?1), (?1 * 3.14), ((?1 % ?2) + 1))").unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 1..=count as i64 {
                    pstmt
                        .execute(rusqlite::params![i, cat_count as i64])
                        .unwrap();
                }
            }
            conn.execute_batch("COMMIT").unwrap();
            conn.execute_batch("CREATE INDEX idx_prod_cat ON products(category_id);")
                .unwrap();
            conn
        };

        let cat_count = (count / 20).max(5);
        let fs_conn = {
            let conn = fsqlite::Connection::open(":memory:").unwrap();
            apply_pragmas_fsqlite(&conn);
            fs_execute(
                &conn,
                "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL, category_id INTEGER)",
            );
            fs_execute(
                &conn,
                "CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT)",
            );
            fs_execute(&conn, "BEGIN");
            #[allow(clippy::cast_possible_wrap)]
            for i in 1..=cat_count as i64 {
                fs_execute(
                    &conn,
                    &format!("INSERT INTO categories VALUES ({i}, 'cat_{i}')"),
                );
            }
            #[allow(clippy::cast_possible_wrap)]
            for i in 1..=count as i64 {
                let cid = (i % cat_count as i64) + 1;
                let price = i as f64 * 3.14;
                fs_execute(
                    &conn,
                    &format!("INSERT INTO products VALUES ({i}, 'prod_{i}', {price}, {cid})"),
                );
            }
            fs_execute(&conn, "COMMIT");
            fs_execute(&conn, "CREATE INDEX idx_prod_cat ON products(category_id)");
            conn
        };

        eprintln!("done.");

        // Scalar subquery in SELECT.
        eprint!("    Scalar subquery in SELECT... ");
        let cs = {
            let mut stmt = cs_conn.prepare(
                "SELECT p.name, (SELECT c.name FROM categories c WHERE c.id = p.category_id) AS cat_name FROM products p LIMIT 100"
            ).unwrap();
            measure(&format!("cs_scalar_sub_{count}"), 100, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare(
                "SELECT p.name, (SELECT c.name FROM categories c WHERE c.id = p.category_id) AS cat_name FROM products p LIMIT 100",
            )
            .unwrap();
        let fs = measure(&format!("fs_scalar_sub_{count}"), 100, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / scalar subquery (LIMIT 100)"),
            Some(cs),
            Some(fs),
        );

        // EXISTS subquery.
        eprint!("    EXISTS subquery... ");
        let half = count / 2;
        let cs = {
            let sql = format!(
                "SELECT COUNT(*) FROM products p WHERE EXISTS (SELECT 1 FROM categories c WHERE c.id = p.category_id AND c.id <= {half})"
            );
            let mut stmt = cs_conn.prepare(&sql).unwrap();
            measure(&format!("cs_exists_{count}"), 1, || {
                let _: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            })
        };
        let fs = {
            let sql = format!(
                "SELECT COUNT(*) FROM products p WHERE EXISTS (SELECT 1 FROM categories c WHERE c.id = p.category_id AND c.id <= {half})"
            );
            let stmt = fs_conn.prepare(&sql).unwrap();
            measure(&format!("fs_exists_{count}"), 1, || {
                let _ = stmt.query_row();
            })
        };
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / EXISTS subquery"),
            Some(cs),
            Some(fs),
        );

        // IN subquery.
        eprint!("    IN subquery... ");
        let cs = {
            let sql = "SELECT COUNT(*) FROM products WHERE category_id IN (SELECT id FROM categories WHERE id <= 5)";
            let mut stmt = cs_conn.prepare(sql).unwrap();
            measure(&format!("cs_in_sub_{count}"), 1, || {
                let _: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT COUNT(*) FROM products WHERE category_id IN (SELECT id FROM categories WHERE id <= 5)")
            .unwrap();
        let fs = measure(&format!("fs_in_sub_{count}"), 1, || {
            let _ = fs_stmt.query_row();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} rows / IN subquery"), Some(cs), Some(fs));

        // CTE (non-recursive).
        eprint!("    CTE (non-recursive)... ");
        let cs = {
            let mut stmt = cs_conn.prepare(
                "WITH top_cats AS (SELECT category_id, SUM(price) AS total FROM products GROUP BY category_id ORDER BY total DESC LIMIT 5) \
                 SELECT p.name, p.price FROM products p JOIN top_cats tc ON p.category_id = tc.category_id"
            ).unwrap();
            measure(&format!("cs_cte_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare(
                "WITH top_cats AS (SELECT category_id, SUM(price) AS total FROM products GROUP BY category_id ORDER BY total DESC LIMIT 5) \
                 SELECT p.name, p.price FROM products p JOIN top_cats tc ON p.category_id = tc.category_id",
            )
            .unwrap();
        let fs = measure(&format!("fs_cte_{count}"), count, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} rows / CTE + JOIN"), Some(cs), Some(fs));
    }

    // Recursive CTE.
    eprint!("    Recursive CTE (generate_series 1..1000)... ");
    let cs = {
        let cs_conn = rusqlite::Connection::open_in_memory().unwrap();
        let mut stmt = cs_conn.prepare(
            "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 1000) SELECT SUM(x) FROM cnt"
        ).unwrap();
        measure("cs_recursive_cte", 1000, || {
            let _: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
        })
    };
    let fs = {
        let fs_conn = fsqlite::Connection::open(":memory:").unwrap();
        let stmt = fs_conn
            .prepare(
                "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 1000) SELECT SUM(x) FROM cnt",
            )
            .unwrap();
        measure("fs_recursive_cte", 1000, || {
            let _ = stmt.query_row();
        })
    };
    eprintln!(
        "C={} F={}",
        format_duration(cs.median()),
        format_duration(fs.median())
    );
    section.add_row("Recursive CTE (1..1000 SUM)", Some(cs), Some(fs));
}

// ─── Section 10: String & LIKE performance ──────────────────────────────

fn bench_string_operations(report: &mut BenchReport, row_counts: &[usize]) {
    let section = report.add_section(
        "String & Pattern Matching Performance",
        "LIKE patterns, string functions, and text-heavy queries.",
    );

    for &count in row_counts {
        if count > 100_000 {
            continue;
        }

        eprint!("  Setting up string table ({count} rows)... ");

        let cs_conn = {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            apply_pragmas_csqlite(&conn);
            conn.execute_batch(
                "CREATE TABLE docs (id INTEGER PRIMARY KEY, title TEXT, body TEXT, tag TEXT);",
            )
            .unwrap();
            conn.execute_batch("BEGIN").unwrap();
            {
                let mut stmt = conn.prepare(
                    "INSERT INTO docs VALUES (?1, ('Document ' || ?1 || ': Important Analysis'), \
                     ('This is the body of document ' || ?1 || '. It contains various keywords like performance, benchmark, analysis, results, and optimization. \
                     The document is about testing and measuring throughput.'), \
                     CASE ?1 % 5 WHEN 0 THEN 'research' WHEN 1 THEN 'report' WHEN 2 THEN 'memo' WHEN 3 THEN 'analysis' ELSE 'draft' END)"
                ).unwrap();
                #[allow(clippy::cast_possible_wrap)]
                for i in 1..=count as i64 {
                    stmt.execute(rusqlite::params![i]).unwrap();
                }
            }
            conn.execute_batch("COMMIT").unwrap();
            conn
        };

        let fs_conn = {
            let conn = fsqlite::Connection::open(":memory:").unwrap();
            apply_pragmas_fsqlite(&conn);
            fs_execute(
                &conn,
                "CREATE TABLE docs (id INTEGER PRIMARY KEY, title TEXT, body TEXT, tag TEXT)",
            );
            fs_execute(&conn, "BEGIN");
            #[allow(clippy::cast_possible_wrap)]
            for i in 1..=count as i64 {
                let tag = match i % 5 {
                    0 => "research",
                    1 => "report",
                    2 => "memo",
                    3 => "analysis",
                    _ => "draft",
                };
                fs_execute(
                    &conn,
                    &format!(
                        "INSERT INTO docs VALUES ({i}, 'Document {i}: Important Analysis', \
                         'This is the body of document {i}. It contains various keywords like performance, benchmark, analysis, results, and optimization. \
                         The document is about testing and measuring throughput.', '{tag}')"
                    ),
                );
            }
            fs_execute(&conn, "COMMIT");
            conn
        };

        eprintln!("done.");

        // LIKE with prefix pattern (sargable).
        eprint!("    LIKE prefix pattern... ");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT COUNT(*) FROM docs WHERE title LIKE 'Document 1%'")
                .unwrap();
            measure(&format!("cs_like_prefix_{count}"), 1, || {
                let _: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT COUNT(*) FROM docs WHERE title LIKE 'Document 1%'")
            .unwrap();
        let fs = measure(&format!("fs_like_prefix_{count}"), 1, || {
            let _ = fs_stmt.query_row();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / LIKE 'prefix%'"),
            Some(cs),
            Some(fs),
        );

        // LIKE with wildcard (full scan).
        eprint!("    LIKE wildcard... ");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT COUNT(*) FROM docs WHERE body LIKE '%benchmark%'")
                .unwrap();
            measure(&format!("cs_like_wild_{count}"), 1, || {
                let _: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT COUNT(*) FROM docs WHERE body LIKE '%benchmark%'")
            .unwrap();
        let fs = measure(&format!("fs_like_wild_{count}"), 1, || {
            let _ = fs_stmt.query_row();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / LIKE '%wildcard%'"),
            Some(cs),
            Some(fs),
        );

        // String functions: LENGTH, UPPER, SUBSTR.
        eprint!("    String functions (LENGTH + UPPER + SUBSTR)... ");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT LENGTH(title), UPPER(tag), SUBSTR(body, 1, 50) FROM docs")
                .unwrap();
            measure(&format!("cs_str_funcs_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT LENGTH(title), UPPER(tag), SUBSTR(body, 1, 50) FROM docs")
            .unwrap();
        let fs = measure(&format!("fs_str_funcs_{count}"), count, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(
            &format!("{count} rows / string functions"),
            Some(cs),
            Some(fs),
        );

        // GROUP_CONCAT.
        eprint!("    GROUP_CONCAT... ");
        let cs = {
            let mut stmt = cs_conn
                .prepare("SELECT tag, GROUP_CONCAT(id, ',') FROM docs GROUP BY tag")
                .unwrap();
            measure(&format!("cs_group_concat_{count}"), count, || {
                let _rows = collect_rusqlite_rows(&mut stmt, []).unwrap();
            })
        };
        let fs_stmt = fs_conn
            .prepare("SELECT tag, GROUP_CONCAT(id, ',') FROM docs GROUP BY tag")
            .unwrap();
        let fs = measure(&format!("fs_group_concat_{count}"), count, || {
            let _ = fs_stmt.query();
        });
        eprintln!(
            "C={} F={}",
            format_duration(cs.median()),
            format_duration(fs.median())
        );
        section.add_row(&format!("{count} rows / GROUP_CONCAT"), Some(cs), Some(fs));
    }
}

// ─── Main ──────────────────────────────────────────────────────────────

fn write_json_report(report: &JsonBenchmarkReport, path: &str) {
    let Ok(json) = serde_json::to_string_pretty(report) else {
        eprintln!("ERROR: Could not serialize JSON report");
        return;
    };

    match std::fs::write(path, format!("{json}\n")) {
        Ok(()) => eprintln!("JSON report written to: {path}"),
        Err(e) => eprintln!("ERROR: Could not write JSON report: {e}"),
    }
}

fn print_json_report(report: &JsonBenchmarkReport) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("ERROR: Could not serialize JSON report: {err}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let options = match parse_cli_args(&args) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("ERROR: {err}");
            print_usage();
            std::process::exit(2);
        }
    };
    if options.print_json_schema {
        print_benchmark_json_schema();
        return;
    }

    let row_counts = if options.quick {
        ROW_COUNTS_QUICK
    } else {
        ROW_COUNTS
    };
    let filter_lower = options.filter.as_ref().map(|filter| filter.to_lowercase());

    let should_run = |name: &str| -> bool {
        match &filter_lower {
            Some(filter) => name.to_lowercase().contains(filter),
            None => true,
        }
    };

    let bench_start = Instant::now();
    let environment = DetectedEnvironment::detect();
    print_run_banner(!options.json_stdout, &options, row_counts, &environment);

    let mut report = BenchReport::new();
    let total_sections = 10;
    let mut section_num = 0;

    // Section 1: Insert by row count across record sizes.
    if should_run("insert") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] INSERT throughput by row count");
        for &record_size in RecordSize::ALL {
            bench_insert_by_row_count(&mut report, row_counts, record_size);
        }
    }

    // Section 2: Transaction strategy comparison.
    if should_run("txn") || should_run("transaction") || should_run("insert") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] Transaction strategy comparison");
        bench_insert_by_txn_strategy(&mut report, row_counts);
    }

    // Section 3: Record size comparison.
    if should_run("record") || should_run("size") || should_run("insert") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] Record size comparison");
        bench_insert_by_record_size(&mut report);
    }

    // Section 4: Concurrent writers.
    if should_run("concurrent") || should_run("writer") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] Concurrent writers");
        bench_concurrent_writers(&mut report);
    }

    // Section 5: Read-after-write.
    if should_run("read") || should_run("query") || should_run("select") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] Read-after-write query performance");
        bench_read_after_write(&mut report, row_counts);
    }

    // Section 6: Update/delete.
    if should_run("update") || should_run("delete") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] UPDATE/DELETE throughput");
        bench_update_delete(&mut report, row_counts);
    }

    // Section 7: Mixed OLTP.
    if should_run("oltp") || should_run("mixed") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] Mixed OLTP workload");
        bench_mixed_oltp(&mut report);
    }

    // Section 8: JOIN performance.
    if should_run("join") || should_run("query") || should_run("select") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] JOIN performance");
        bench_join_performance(&mut report, row_counts);
    }

    // Section 9: Subquery & CTE.
    if should_run("subquery") || should_run("cte") || should_run("query") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] Subquery & CTE performance");
        bench_subquery_cte(&mut report, row_counts);
    }

    // Section 10: String operations.
    if should_run("string") || should_run("like") || should_run("pattern") {
        section_num += 1;
        eprintln!("\n[{section_num}/{total_sections}] String & pattern matching");
        bench_string_operations(&mut report, row_counts);
    }

    let total_elapsed = bench_start.elapsed();
    eprintln!(
        "\nBenchmark complete in {:.1}s. Generating reports...",
        total_elapsed.as_secs_f64()
    );

    if !options.json_stdout {
        report.print(total_elapsed, &environment);
    }

    let html_file = if options.emit_html {
        Some(
            options
                .html_path
                .clone()
                .unwrap_or_else(|| timestamp_filename("benchmark_report", "html")),
        )
    } else {
        None
    };
    if let Some(path) = html_file.as_deref() {
        report.write_html(path);
    }

    let json_file = if let Some(path) = options.json_out_path.clone() {
        Some(path)
    } else if options.emit_timestamped_json {
        Some(timestamp_filename("benchmark_report", "json"))
    } else {
        None
    };

    if json_file.is_some() || options.json_stdout {
        let json_report = build_json_report(
            &report,
            total_elapsed,
            JsonRunConfig {
                quick: options.quick,
                filter: options.filter.clone(),
                warmup_iterations: WARMUP_ITERS,
                min_iterations: MIN_ITERS,
                max_iterations: MAX_ITERS,
                target_duration_secs: TARGET_DURATION.as_secs(),
                row_counts: row_counts.to_vec(),
                html_output_path: html_file.clone(),
                json_output_path: json_file.clone(),
                json_stdout: options.json_stdout,
            },
            environment.clone(),
        );

        if let Some(path) = json_file.as_deref() {
            write_json_report(&json_report, path);
        }
        if options.json_stdout {
            print_json_report(&json_report);
        }
    }
}
