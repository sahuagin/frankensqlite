//! Narrow profiling binary for UPDATE/DELETE fsqlite hot path.
//!
//! Runs the same fsqlite UPDATE/DELETE workload as `comprehensive-bench`'s
//! Section 6, but without the C SQLite comparison or the benchmark reporting
//! ceremony, so perf/flamegraph stacks stay focused on the fsqlite engine.
//!
//! Usage:
//!   perf-update-delete                         # default: 10_000 rows, 10 iters, update+delete, fsqlite only
//!   perf-update-delete 100000 3 update
//!   perf-update-delete 1000   5 delete compare
//!
//! Arguments:
//!   [rows]   Number of rows to pre-populate (default 10_000)
//!   [iters]  Number of outer iterations for profiling (default 10)
//!   [which]  "update" | "delete" | "both" (default "both")
//!   [engine] "fsqlite" | "sqlite" | "compare" (default "fsqlite")

use std::fmt;
use std::process::ExitCode;
use std::time::Instant;

const DEFAULT_ROWS: usize = 10_000;
const DEFAULT_ITERS: usize = 10;
const BENCH_CREATE_SQL: &str =
    "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT NOT NULL, value REAL NOT NULL)";
const BENCH_INSERT_SQL: &str = "INSERT INTO bench VALUES (?1, ('user_' || ?1), (?1 * 0.137))";
const BENCHMARK_PRAGMAS: &[&str] = &[
    "PRAGMA page_size = 4096;",
    "PRAGMA journal_mode = WAL;",
    "PRAGMA synchronous = NORMAL;",
    "PRAGMA cache_size = -64000;",
    // This profiler never issues FOR SYSTEM_TIME queries. Match
    // comprehensive_bench's write scenarios and suppress the optional
    // MemDatabase history clone that otherwise runs on each explicit COMMIT.
    "PRAGMA fsqlite_capture_time_travel_snapshots=false;",
];
const CSQLITE_BENCHMARK_PRAGMAS: &[&str] = &[
    "PRAGMA page_size = 4096;",
    "PRAGMA journal_mode = WAL;",
    "PRAGMA synchronous = NORMAL;",
    "PRAGMA cache_size = -64000;",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkloadKind {
    Update,
    Delete,
    Both,
}

impl WorkloadKind {
    fn parse(raw: &str) -> Result<Self, RunError> {
        match raw {
            "update" => Ok(Self::Update),
            "delete" => Ok(Self::Delete),
            "both" => Ok(Self::Both),
            other => Err(RunError::Usage(format!(
                "invalid workload '{other}'; expected update, delete, or both"
            ))),
        }
    }

    fn do_update(self) -> bool {
        matches!(self, Self::Update | Self::Both)
    }

    fn do_delete(self) -> bool {
        matches!(self, Self::Delete | Self::Both)
    }
}

impl fmt::Display for WorkloadKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Update => f.write_str("update"),
            Self::Delete => f.write_str("delete"),
            Self::Both => f.write_str("both"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EngineKind {
    Fsqlite,
    Sqlite,
    Compare,
}

impl EngineKind {
    fn parse(raw: &str) -> Result<Self, RunError> {
        match raw {
            "fsqlite" => Ok(Self::Fsqlite),
            "sqlite" => Ok(Self::Sqlite),
            "compare" => Ok(Self::Compare),
            other => Err(RunError::Usage(format!(
                "invalid engine '{other}'; expected fsqlite, sqlite, or compare"
            ))),
        }
    }

    fn run_fsqlite(self) -> bool {
        matches!(self, Self::Fsqlite | Self::Compare)
    }

    fn run_sqlite(self) -> bool {
        matches!(self, Self::Sqlite | Self::Compare)
    }
}

impl fmt::Display for EngineKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fsqlite => f.write_str("fsqlite"),
            Self::Sqlite => f.write_str("sqlite"),
            Self::Compare => f.write_str("compare"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BenchArgs {
    rows: usize,
    iters: usize,
    workload: WorkloadKind,
    engine: EngineKind,
}

#[derive(Debug, PartialEq, Eq)]
enum RunError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => f.write_str(message),
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("perf-update-delete: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), RunError> {
    let args = parse_args(std::env::args().skip(1))?;
    run_benchmark(&args)
}

fn parse_args<I>(args: I) -> Result<BenchArgs, RunError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();

    let rows = match args.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| {
            RunError::Usage(format!(
                "invalid rows '{raw}'; expected a non-negative integer"
            ))
        })?,
        None => DEFAULT_ROWS,
    };
    let iters = match args.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| {
            RunError::Usage(format!(
                "invalid iters '{raw}'; expected a non-negative integer"
            ))
        })?,
        None => DEFAULT_ITERS,
    };
    let workload = match args.next() {
        Some(raw) => WorkloadKind::parse(&raw)?,
        None => WorkloadKind::Both,
    };
    let engine = match args.next() {
        Some(raw) => EngineKind::parse(&raw)?,
        None => EngineKind::Fsqlite,
    };
    if let Some(extra) = args.next() {
        return Err(RunError::Usage(format!(
            "unexpected extra argument '{extra}'; usage: perf-update-delete [rows] [iters] [update|delete|both] [fsqlite|sqlite|compare]"
        )));
    }
    if iters == 0 {
        return Err(RunError::Usage(
            "iters must be greater than zero".to_string(),
        ));
    }

    Ok(BenchArgs {
        rows,
        iters,
        workload,
        engine,
    })
}

fn per_row_ns(total_ns: u128, op_count: usize, iters: usize) -> f64 {
    let total_ops = op_count.saturating_mul(iters);
    if total_ops == 0 {
        0.0
    } else {
        total_ns as f64 / total_ops as f64
    }
}

fn apply_benchmark_pragmas(conn: &fsqlite::Connection) -> Result<(), RunError> {
    for pragma in BENCHMARK_PRAGMAS {
        conn.execute(pragma)
            .map_err(|err| RunError::Runtime(format!("apply benchmark pragma {pragma}: {err}")))?;
    }

    if std::env::var("FSQLITE_BENCH_LAB_UNSAFE")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        for pragma in [
            "PRAGMA fsqlite.write_merge = LAB_UNSAFE;",
            "PRAGMA fsqlite.ssi_e_process_alpha = 0.001;",
        ] {
            conn.execute(pragma).map_err(|err| {
                RunError::Runtime(format!("apply benchmark pragma {pragma}: {err}"))
            })?;
        }
    }

    Ok(())
}

fn apply_csqlite_benchmark_pragmas(conn: &rusqlite::Connection) -> Result<(), RunError> {
    for pragma in CSQLITE_BENCHMARK_PRAGMAS {
        conn.execute_batch(pragma).map_err(|err| {
            RunError::Runtime(format!("apply C SQLite benchmark pragma {pragma}: {err}"))
        })?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TimingTotals {
    total: u128,
    populate: u128,
    update: u128,
    delete: u128,
}

fn run_benchmark(args: &BenchArgs) -> Result<(), RunError> {
    let rows_i64 = i64::try_from(args.rows)
        .map_err(|_| RunError::Usage("rows must fit within i64".to_string()))?;
    let update_count = args.rows / 10;
    let delete_count = args.rows / 20;

    eprintln!(
        "perf-update-delete: rows={} iters={} which={} engine={} (do_update={} do_delete={} update_count={} delete_count={})",
        args.rows,
        args.iters,
        args.workload,
        args.engine,
        args.workload.do_update(),
        args.workload.do_delete(),
        update_count,
        delete_count,
    );

    let mut fsqlite_totals = None;
    let mut sqlite_totals = None;

    if args.engine.run_fsqlite() {
        let totals = run_fsqlite_benchmark(args, rows_i64, update_count, delete_count)?;
        print_engine_summary("fsqlite", args, update_count, delete_count, totals);
        fsqlite_totals = Some(totals);
    }

    if args.engine.run_sqlite() {
        let totals = run_sqlite_benchmark(args, rows_i64, update_count, delete_count)?;
        print_engine_summary("sqlite", args, update_count, delete_count, totals);
        sqlite_totals = Some(totals);
    }

    if let (Some(fsqlite), Some(sqlite)) = (fsqlite_totals, sqlite_totals) {
        print_comparison_summary(args, fsqlite, sqlite);
    }

    Ok(())
}

fn run_fsqlite_benchmark(
    args: &BenchArgs,
    rows_i64: i64,
    update_count: usize,
    delete_count: usize,
) -> Result<TimingTotals, RunError> {
    let t_all = Instant::now();
    let mut total_update_ns: u128 = 0;
    let mut total_delete_ns: u128 = 0;
    let mut total_populate_ns: u128 = 0;

    for iter in 0..args.iters {
        let conn = fsqlite::Connection::open(":memory:")
            .map_err(|err| RunError::Runtime(format!("open in-memory database: {err}")))?;
        apply_benchmark_pragmas(&conn)?;
        conn.execute(BENCH_CREATE_SQL)
            .map_err(|err| RunError::Runtime(format!("create benchmark table: {err}")))?;
        conn.execute("BEGIN")
            .map_err(|err| RunError::Runtime(format!("begin populate transaction: {err}")))?;
        let stmt = conn
            .prepare(BENCH_INSERT_SQL)
            .map_err(|err| RunError::Runtime(format!("prepare populate statement: {err}")))?;
        let t0 = Instant::now();
        for i in 0..rows_i64 {
            stmt.execute_with_params(&[fsqlite::SqliteValue::Integer(i)])
                .map_err(|err| RunError::Runtime(format!("populate row {i}: {err}")))?;
        }
        conn.execute("COMMIT")
            .map_err(|err| RunError::Runtime(format!("commit populate transaction: {err}")))?;
        total_populate_ns += t0.elapsed().as_nanos();

        if args.workload.do_update() {
            conn.execute("BEGIN")
                .map_err(|err| RunError::Runtime(format!("begin update transaction: {err}")))?;
            let update = conn
                .prepare("UPDATE bench SET value = ?2 WHERE id = ?1")
                .map_err(|err| RunError::Runtime(format!("prepare update statement: {err}")))?;
            let t0 = Instant::now();
            for i in 0..update_count {
                let id = i64::try_from(i).map_err(|_| {
                    RunError::Usage("update_count index overflowed i64".to_string())
                })? * 10;
                update
                    .execute_with_params(&[
                        fsqlite::SqliteValue::Integer(id),
                        fsqlite::SqliteValue::Float(999.99),
                    ])
                    .map_err(|err| RunError::Runtime(format!("update row {id}: {err}")))?;
            }
            conn.execute("COMMIT")
                .map_err(|err| RunError::Runtime(format!("commit update transaction: {err}")))?;
            total_update_ns += t0.elapsed().as_nanos();
        }

        if args.workload.do_delete() {
            conn.execute("BEGIN")
                .map_err(|err| RunError::Runtime(format!("begin delete transaction: {err}")))?;
            let delete = conn
                .prepare("DELETE FROM bench WHERE id = ?1")
                .map_err(|err| RunError::Runtime(format!("prepare delete statement: {err}")))?;
            let t0 = Instant::now();
            for i in 0..delete_count {
                let id = i64::try_from(i).map_err(|_| {
                    RunError::Usage("delete_count index overflowed i64".to_string())
                })? * 20;
                delete
                    .execute_with_params(&[fsqlite::SqliteValue::Integer(id)])
                    .map_err(|err| RunError::Runtime(format!("delete row {id}: {err}")))?;
            }
            conn.execute("COMMIT")
                .map_err(|err| RunError::Runtime(format!("commit delete transaction: {err}")))?;
            total_delete_ns += t0.elapsed().as_nanos();
        }

        if iter == 0 {
            eprintln!("  (first iter complete)");
        }
    }

    Ok(TimingTotals {
        total: t_all.elapsed().as_nanos(),
        populate: total_populate_ns,
        update: total_update_ns,
        delete: total_delete_ns,
    })
}

fn run_sqlite_benchmark(
    args: &BenchArgs,
    rows_i64: i64,
    update_count: usize,
    delete_count: usize,
) -> Result<TimingTotals, RunError> {
    let t_all = Instant::now();
    let mut total_update_ns: u128 = 0;
    let mut total_delete_ns: u128 = 0;
    let mut total_populate_ns: u128 = 0;

    for iter in 0..args.iters {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|err| RunError::Runtime(format!("open C SQLite in-memory database: {err}")))?;
        apply_csqlite_benchmark_pragmas(&conn)?;
        conn.execute(BENCH_CREATE_SQL, [])
            .map_err(|err| RunError::Runtime(format!("create C SQLite benchmark table: {err}")))?;
        conn.execute_batch("BEGIN").map_err(|err| {
            RunError::Runtime(format!("begin C SQLite populate transaction: {err}"))
        })?;
        let mut stmt = conn.prepare(BENCH_INSERT_SQL).map_err(|err| {
            RunError::Runtime(format!("prepare C SQLite populate statement: {err}"))
        })?;
        let t0 = Instant::now();
        for i in 0..rows_i64 {
            stmt.execute(rusqlite::params![i])
                .map_err(|err| RunError::Runtime(format!("populate C SQLite row {i}: {err}")))?;
        }
        conn.execute_batch("COMMIT").map_err(|err| {
            RunError::Runtime(format!("commit C SQLite populate transaction: {err}"))
        })?;
        total_populate_ns += t0.elapsed().as_nanos();

        if args.workload.do_update() {
            conn.execute_batch("BEGIN").map_err(|err| {
                RunError::Runtime(format!("begin C SQLite update transaction: {err}"))
            })?;
            let mut update = conn
                .prepare("UPDATE bench SET value = ?2 WHERE id = ?1")
                .map_err(|err| {
                    RunError::Runtime(format!("prepare C SQLite update statement: {err}"))
                })?;
            let t0 = Instant::now();
            for i in 0..update_count {
                let id = i64::try_from(i).map_err(|_| {
                    RunError::Usage("update_count index overflowed i64".to_string())
                })? * 10;
                update
                    .execute(rusqlite::params![id, 999.99])
                    .map_err(|err| RunError::Runtime(format!("update C SQLite row {id}: {err}")))?;
            }
            conn.execute_batch("COMMIT").map_err(|err| {
                RunError::Runtime(format!("commit C SQLite update transaction: {err}"))
            })?;
            total_update_ns += t0.elapsed().as_nanos();
        }

        if args.workload.do_delete() {
            conn.execute_batch("BEGIN").map_err(|err| {
                RunError::Runtime(format!("begin C SQLite delete transaction: {err}"))
            })?;
            let mut delete = conn
                .prepare("DELETE FROM bench WHERE id = ?1")
                .map_err(|err| {
                    RunError::Runtime(format!("prepare C SQLite delete statement: {err}"))
                })?;
            let t0 = Instant::now();
            for i in 0..delete_count {
                let id = i64::try_from(i).map_err(|_| {
                    RunError::Usage("delete_count index overflowed i64".to_string())
                })? * 20;
                delete
                    .execute(rusqlite::params![id])
                    .map_err(|err| RunError::Runtime(format!("delete C SQLite row {id}: {err}")))?;
            }
            conn.execute_batch("COMMIT").map_err(|err| {
                RunError::Runtime(format!("commit C SQLite delete transaction: {err}"))
            })?;
            total_delete_ns += t0.elapsed().as_nanos();
        }

        if iter == 0 {
            eprintln!("  (first sqlite iter complete)");
        }
    }

    Ok(TimingTotals {
        total: t_all.elapsed().as_nanos(),
        populate: total_populate_ns,
        update: total_update_ns,
        delete: total_delete_ns,
    })
}

fn print_engine_summary(
    engine: &str,
    args: &BenchArgs,
    update_count: usize,
    delete_count: usize,
    totals: TimingTotals,
) {
    let per_row_update = if args.workload.do_update() {
        per_row_ns(totals.update, update_count, args.iters)
    } else {
        0.0
    };
    let per_row_delete = if args.workload.do_delete() {
        per_row_ns(totals.delete, delete_count, args.iters)
    } else {
        0.0
    };
    eprintln!(
        "{engine}: total={}ms populate={}ms update={}ms delete={}ms  |  \
        per-row-update={per_row_update:.0}ns  per-row-delete={per_row_delete:.0}ns",
        totals.total / 1_000_000,
        totals.populate / 1_000_000,
        totals.update / 1_000_000,
        totals.delete / 1_000_000,
    );
}

fn ratio_or_zero(numerator: u128, denominator: u128) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn print_comparison_summary(args: &BenchArgs, fsqlite: TimingTotals, sqlite: TimingTotals) {
    let update_ratio = if args.workload.do_update() {
        ratio_or_zero(fsqlite.update, sqlite.update)
    } else {
        0.0
    };
    let delete_ratio = if args.workload.do_delete() {
        ratio_or_zero(fsqlite.delete, sqlite.delete)
    } else {
        0.0
    };
    eprintln!(
        "fsqlite/sqlite time ratio: total={:.2}x populate={:.2}x update={update_ratio:.2}x delete={delete_ratio:.2}x",
        ratio_or_zero(fsqlite.total, sqlite.total),
        ratio_or_zero(fsqlite.populate, sqlite.populate),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        BENCH_CREATE_SQL, BENCH_INSERT_SQL, BENCHMARK_PRAGMAS, BenchArgs, DEFAULT_ITERS,
        DEFAULT_ROWS, EngineKind, RunError, WorkloadKind, parse_args, per_row_ns, run_benchmark,
    };

    #[test]
    fn parse_args_uses_defaults() {
        assert_eq!(
            parse_args(std::iter::empty::<String>()).unwrap(),
            BenchArgs {
                rows: DEFAULT_ROWS,
                iters: DEFAULT_ITERS,
                workload: WorkloadKind::Both,
                engine: EngineKind::Fsqlite,
            }
        );
    }

    #[test]
    fn parse_args_rejects_invalid_workload() {
        let err = parse_args(["100".to_string(), "2".to_string(), "bogus".to_string()])
            .expect_err("invalid workload should fail");
        assert_eq!(
            err,
            RunError::Usage(
                "invalid workload 'bogus'; expected update, delete, or both".to_string()
            )
        );
    }

    #[test]
    fn parse_args_rejects_zero_iters() {
        let err =
            parse_args(["100".to_string(), "0".to_string()]).expect_err("zero iters should fail");
        assert_eq!(
            err,
            RunError::Usage("iters must be greater than zero".to_string())
        );
    }

    #[test]
    fn per_row_ns_returns_zero_for_zero_ops() {
        assert_eq!(per_row_ns(50_000, 0, 5), 0.0);
        assert_eq!(per_row_ns(50_000, 3, 0), 0.0);
    }

    #[test]
    fn parse_args_accepts_small_row_counts() {
        assert_eq!(
            parse_args(["5".to_string(), "1".to_string(), "update".to_string()]).unwrap(),
            BenchArgs {
                rows: 5,
                iters: 1,
                workload: WorkloadKind::Update,
                engine: EngineKind::Fsqlite,
            }
        );
    }

    #[test]
    fn parse_args_accepts_compare_engine() {
        assert_eq!(
            parse_args([
                "5".to_string(),
                "1".to_string(),
                "both".to_string(),
                "compare".to_string(),
            ])
            .unwrap(),
            BenchArgs {
                rows: 5,
                iters: 1,
                workload: WorkloadKind::Both,
                engine: EngineKind::Compare,
            }
        );
    }

    #[test]
    fn parse_args_rejects_invalid_engine() {
        let err = parse_args([
            "100".to_string(),
            "2".to_string(),
            "both".to_string(),
            "bogus".to_string(),
        ])
        .expect_err("invalid engine should fail");
        assert_eq!(
            err,
            RunError::Usage(
                "invalid engine 'bogus'; expected fsqlite, sqlite, or compare".to_string()
            )
        );
    }

    #[test]
    fn benchmark_schema_matches_small_record_workload() {
        assert_eq!(
            BENCH_CREATE_SQL,
            "CREATE TABLE bench (id INTEGER PRIMARY KEY, name TEXT NOT NULL, value REAL NOT NULL)"
        );
        assert_eq!(
            BENCH_INSERT_SQL,
            "INSERT INTO bench VALUES (?1, ('user_' || ?1), (?1 * 0.137))"
        );
    }

    #[test]
    fn benchmark_pragmas_disable_time_travel_capture() {
        assert!(
            BENCHMARK_PRAGMAS.iter().any(|pragma| pragma
                .eq_ignore_ascii_case("PRAGMA fsqlite_capture_time_travel_snapshots=false;")),
            "perf-update-delete should profile UPDATE/DELETE, not optional time-travel snapshot cloning"
        );
    }

    #[test]
    fn run_benchmark_smoke_small_workload() {
        let args = BenchArgs {
            rows: 5,
            iters: 1,
            workload: WorkloadKind::Both,
            engine: EngineKind::Compare,
        };
        run_benchmark(&args).expect("small smoke workload should succeed");
    }
}
