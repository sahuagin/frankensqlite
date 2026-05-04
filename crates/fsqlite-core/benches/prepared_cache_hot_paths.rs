use std::env;
use std::hint::black_box;
use std::time::Instant;

use fsqlite_core::connection::Connection;
use fsqlite_types::SqliteValue;
use tempfile::NamedTempFile;

const INSERT_SQL: &str = "INSERT INTO bench (id, payload) VALUES (?1, ?2)";
const SELECT_COUNT_SUM_SQL: &str = "SELECT COUNT(*), SUM(score) FROM select_bench";
const SELECT_COVERING_INDEX_SQL: &str = "SELECT name FROM select_bench WHERE name = ?1";
const COUNT_INDEXED_ROWID_PROBE_SQL: &str =
    "SELECT COUNT(*) FROM products WHERE category_id IN (SELECT id FROM categories WHERE id <= 5)";

fn open_mt_mvcc_prepare_conn() -> (Connection, NamedTempFile) {
    let tmp = NamedTempFile::new().expect("tempfile");
    let path = tmp
        .path()
        .to_str()
        .expect("tempfile path must be utf-8")
        .to_owned();
    let conn = Connection::open(path).expect("open connection");
    conn.execute("CREATE TABLE bench (id INTEGER PRIMARY KEY, payload TEXT);")
        .expect("create table");
    conn.execute("BEGIN;").expect("begin transaction");
    (conn, tmp)
}

fn bench_mt_mvcc_prepare_hit(iterations: u64) -> f64 {
    let (conn, _tmp) = open_mt_mvcc_prepare_conn();
    let warmed = conn.prepare(INSERT_SQL).expect("warm prepare");
    black_box(&warmed);

    let start = Instant::now();
    for _ in 0..iterations {
        let stmt = conn.prepare(black_box(INSERT_SQL)).expect("prepare hit");
        black_box(stmt);
    }
    start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn bench_mt_mvcc_prepare_then_execute_cycle(iterations: u64) -> f64 {
    let (conn, _tmp) = open_mt_mvcc_prepare_conn();
    let warmed = conn.prepare(INSERT_SQL).expect("warm prepare");
    let warmed_params = [
        SqliteValue::Integer(0),
        SqliteValue::Text(String::from("warmup").into()),
    ];
    warmed
        .execute_with_params(&warmed_params)
        .expect("warm execute");
    black_box(&warmed);

    let start = Instant::now();
    for row_id in 1..=iterations {
        let stmt = conn.prepare(black_box(INSERT_SQL)).expect("prepare hit");
        let params = [
            SqliteValue::Integer(i64::try_from(row_id).expect("row id fits i64")),
            SqliteValue::Text(format!("payload_{row_id}").into()),
        ];
        let inserted = stmt.execute_with_params(&params).expect("execute");
        black_box(inserted);
    }
    start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn open_prepared_select_fast_path_conn() -> Connection {
    let conn = Connection::open(":memory:").expect("open memory connection");
    conn.execute(
        "CREATE TABLE select_bench (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            score INTEGER NOT NULL
        );",
    )
    .expect("create select bench table");
    conn.execute("CREATE INDEX select_bench_name ON select_bench(name);")
        .expect("create select bench index");
    let insert = conn
        .prepare("INSERT INTO select_bench VALUES (?1, ?2, ?3)")
        .expect("prepare select bench insert");
    for id in 1..=64_i64 {
        insert
            .execute_with_params(&[
                SqliteValue::Integer(id),
                SqliteValue::Text(format!("name_{id}").into()),
                SqliteValue::Integer(id * 7),
            ])
            .expect("seed select bench row");
    }
    conn
}

fn bench_prepared_select_fast_path_pair(iterations: u64) -> f64 {
    let conn = open_prepared_select_fast_path_conn();
    let count_sum = conn
        .prepare(SELECT_COUNT_SUM_SQL)
        .expect("prepare count/sum");
    let covering_index = conn
        .prepare(SELECT_COVERING_INDEX_SQL)
        .expect("prepare covering indexed equality");
    let probe = [SqliteValue::Text("name_32".into())];
    black_box(count_sum.query_row().expect("warm count/sum"));
    black_box(
        covering_index
            .query_with_params(&probe)
            .expect("warm covering indexed equality"),
    );

    let start = Instant::now();
    for _ in 0..iterations {
        black_box(count_sum.query_row().expect("count/sum fast path"));
        black_box(
            covering_index
                .query_with_params(&probe)
                .expect("covering indexed equality fast path"),
        );
    }
    start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn open_prepared_count_indexed_rowid_probe_conn(count: i64) -> Connection {
    let conn = Connection::open(":memory:").expect("open memory connection");
    for pragma in [
        "PRAGMA page_size = 4096;",
        "PRAGMA journal_mode = WAL;",
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA cache_size = -64000;",
        "PRAGMA fsqlite_capture_time_travel_snapshots=false;",
    ] {
        let _ = conn.execute(pragma);
    }
    conn.execute(
        "CREATE TABLE products (
            id INTEGER PRIMARY KEY,
            name TEXT,
            price REAL,
            category_id INTEGER
        );",
    )
    .expect("create products table");
    conn.execute("CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT);")
        .expect("create categories table");
    conn.execute("BEGIN;").expect("begin fixture transaction");
    let cat_count = (count / 20).max(5);
    for id in 1..=cat_count {
        conn.execute(&format!("INSERT INTO categories VALUES ({id}, 'cat_{id}')"))
            .expect("seed category row");
    }
    for id in 1..=count {
        let cid = (id % cat_count) + 1;
        let price = id as f64 * 3.14;
        conn.execute(&format!(
            "INSERT INTO products VALUES ({id}, 'prod_{id}', {price}, {cid})"
        ))
        .expect("seed product row");
    }
    conn.execute("COMMIT;").expect("commit fixture transaction");
    conn.execute("CREATE INDEX idx_prod_cat ON products(category_id);")
        .expect("create products category index");
    conn
}

fn bench_prepared_count_indexed_rowid_probe_query_row(iterations: u64, count: i64) -> f64 {
    let conn = open_prepared_count_indexed_rowid_probe_conn(count);
    let stmt = conn
        .prepare(COUNT_INDEXED_ROWID_PROBE_SQL)
        .expect("prepare count indexed rowid probe");
    black_box(stmt.query_row().expect("warm count indexed rowid probe"));
    black_box(
        stmt.query_row()
            .expect("warm cached count indexed rowid probe"),
    );

    let start = Instant::now();
    let mut count_sum = 0_i64;
    for _ in 0..iterations {
        let row = stmt
            .query_row()
            .expect("count indexed rowid probe fast path");
        let Some(SqliteValue::Integer(count)) = row.get(0) else {
            panic!("count indexed rowid probe returned non-integer row: {row:?}");
        };
        count_sum = count_sum.saturating_add(*count);
    }
    black_box(count_sum);
    start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn parse_iterations() -> u64 {
    let mut args = env::args().skip(1);
    let mut iterations = 2_000_000_u64;
    let mut filter = None;
    while let Some(arg) = args.next() {
        if arg == "--iterations" {
            if let Some(value) = args.next() {
                match value.parse() {
                    Ok(parsed) => iterations = parsed,
                    Err(_) => {
                        eprintln!("invalid --iterations value: {value}");
                        std::process::exit(2);
                    }
                }
            }
        } else if arg == "--filter" {
            filter = args.next();
        }
    }
    if let Some(filter) = filter {
        match filter.as_str() {
            "prepare_hit" => {
                let prepare_hit_ns = bench_mt_mvcc_prepare_hit(iterations);
                println!(
                    "prepared_cache_hot_paths mt_mvcc_prepare_hit_ns_per_op={prepare_hit_ns:.2} iterations={iterations}"
                );
                std::process::exit(0);
            }
            "prepare_execute" => {
                let prepare_execute_ns =
                    bench_mt_mvcc_prepare_then_execute_cycle(iterations.min(200_000));
                println!(
                    "prepared_cache_hot_paths mt_mvcc_prepare_then_execute_cycle_ns_per_op={prepare_execute_ns:.2} iterations={}",
                    iterations.min(200_000)
                );
                std::process::exit(0);
            }
            "select_fast_paths" => {
                let select_fast_paths_ns =
                    bench_prepared_select_fast_path_pair(iterations.min(200_000));
                println!(
                    "prepared_cache_hot_paths select_count_sum_plus_covering_index_ns_per_pair={select_fast_paths_ns:.2} iterations={}",
                    iterations.min(200_000)
                );
                std::process::exit(0);
            }
            "count_indexed_rowid_probe" => {
                let count_indexed_rowid_probe_ns =
                    bench_prepared_count_indexed_rowid_probe_query_row(
                        iterations.min(200_000),
                        1_000,
                    );
                println!(
                    "prepared_cache_hot_paths count_indexed_rowid_probe_query_row_ns_per_op={count_indexed_rowid_probe_ns:.2} rows=1000 iterations={}",
                    iterations.min(200_000)
                );
                std::process::exit(0);
            }
            _ => {
                eprintln!("invalid --filter value: {filter}");
                std::process::exit(2);
            }
        }
    }
    iterations
}

fn main() {
    let iterations = parse_iterations();
    let prepare_hit_ns = bench_mt_mvcc_prepare_hit(iterations);
    let prepare_execute_ns = bench_mt_mvcc_prepare_then_execute_cycle(iterations.min(200_000));
    let select_fast_paths_ns = bench_prepared_select_fast_path_pair(iterations.min(200_000));
    let count_indexed_rowid_probe_ns =
        bench_prepared_count_indexed_rowid_probe_query_row(iterations.min(200_000), 1_000);

    println!(
        "prepared_cache_hot_paths mt_mvcc_prepare_hit_ns_per_op={prepare_hit_ns:.2} iterations={iterations}"
    );
    println!(
        "prepared_cache_hot_paths mt_mvcc_prepare_then_execute_cycle_ns_per_op={prepare_execute_ns:.2} iterations={}",
        iterations.min(200_000)
    );
    println!(
        "prepared_cache_hot_paths select_count_sum_plus_covering_index_ns_per_pair={select_fast_paths_ns:.2} iterations={}",
        iterations.min(200_000)
    );
    println!(
        "prepared_cache_hot_paths count_indexed_rowid_probe_query_row_ns_per_op={count_indexed_rowid_probe_ns:.2} rows=1000 iterations={}",
        iterations.min(200_000)
    );
}
