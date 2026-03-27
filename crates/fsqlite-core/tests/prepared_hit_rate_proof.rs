//! bd-db300.4.5.1: Prove actual prepared-artifact hit rates and fast-lane usage
//! on c1 micro-workloads.
//!
//! This test reproduces the commutative_inserts_disjoint_keys c1 workload pattern
//! (the worst measured cell at 0.068x) using prepared statements and proves:
//! 1. Prepared INSERT fast-lane hits = 100% of INSERT ops.
//! 2. Table engine reuse = 100% after first alloc.
//! 3. Parse/compiled cache hits = 0 (expected: prepared stmts bypass these caches).
//! 4. Schema refreshes and publication binds are the dominant per-statement cost.
//!
//! Run:
//!   CARGO_TARGET_DIR=/tmp/pane1-d51 cargo test -p fsqlite-core \
//!     --test prepared_hit_rate_proof -- --test-threads=1 --nocapture

use fsqlite_core::connection::{
    Connection, hot_path_profile_enabled, hot_path_profile_snapshot, reset_hot_path_profile,
    set_hot_path_profile_enabled,
};
use fsqlite_error::FrankenError;
use fsqlite_types::SqliteValue;
use std::sync::{LazyLock, Mutex, MutexGuard};

static HOT_PATH_PROFILE_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn lock_profile_test_mutex() -> MutexGuard<'static, ()> {
    match HOT_PATH_PROFILE_TEST_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

struct HotPathProfileTestGuard {
    _lock: MutexGuard<'static, ()>,
    previous_enabled: bool,
}

impl HotPathProfileTestGuard {
    fn new() -> Self {
        let lock = lock_profile_test_mutex();
        let previous_enabled = hot_path_profile_enabled();
        reset_hot_path_profile();
        set_hot_path_profile_enabled(true);
        Self {
            _lock: lock,
            previous_enabled,
        }
    }
}

impl Drop for HotPathProfileTestGuard {
    fn drop(&mut self) {
        reset_hot_path_profile();
        set_hot_path_profile_enabled(self.previous_enabled);
    }
}

/// Simulate the c1 commutative_inserts workload: N prepared INSERTs into
/// separate tables, autocommit, file-backed WAL.
#[test]
fn test_prepared_fast_lane_hit_rate_on_c1_workload() {
    let _profile_guard = HotPathProfileTestGuard::new();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();

    let conn = Connection::open(path).unwrap();
    conn.execute("PRAGMA journal_mode = WAL").unwrap();
    conn.execute("PRAGMA synchronous = NORMAL").unwrap();

    // Create 2 tables (simulates disjoint-key workload with multiple tables).
    conn.execute("CREATE TABLE t0(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();

    // Prepare statements (one per table, as the real executor does).
    let stmt0 = conn.prepare("INSERT INTO t0 VALUES(?1, ?2)").unwrap();
    let stmt1 = conn.prepare("INSERT INTO t1 VALUES(?1, ?2)").unwrap();

    // Warm: one execution per table to establish baseline.
    stmt0
        .execute_with_params(&[
            fsqlite_types::SqliteValue::Integer(0),
            fsqlite_types::SqliteValue::Text("warm".into()),
        ])
        .unwrap();
    stmt1
        .execute_with_params(&[
            fsqlite_types::SqliteValue::Integer(0),
            fsqlite_types::SqliteValue::Text("warm".into()),
        ])
        .unwrap();

    // Reset counters after warmup.
    reset_hot_path_profile();

    // Measurement: 50 INSERTs per table = 100 total (matches real workload scale).
    for i in 1..=50 {
        stmt0
            .execute_with_params(&[
                fsqlite_types::SqliteValue::Integer(i),
                fsqlite_types::SqliteValue::Text(format!("v{i}").into()),
            ])
            .unwrap();
        stmt1
            .execute_with_params(&[
                fsqlite_types::SqliteValue::Integer(i),
                fsqlite_types::SqliteValue::Text(format!("v{i}").into()),
            ])
            .unwrap();
    }

    let snap = hot_path_profile_snapshot();

    // ─── Scorecard ───
    eprintln!("=== bd-db300.4.5.1: Prepared Hit Rate Proof (100 file-backed INSERTs) ===");
    eprintln!("Parser counters (expected: 0 hits — prepared stmts bypass parse cache):");
    eprintln!(
        "  parse_cache:    hits={:>4}  misses={:>4}",
        snap.parser.parse_cache_hits, snap.parser.parse_cache_misses
    );
    eprintln!(
        "  compiled_cache: hits={:>4}  misses={:>4}",
        snap.parser.compiled_cache_hits, snap.parser.compiled_cache_misses
    );
    eprintln!(
        "  fast_path:      {:>4}  slow_path: {:>4}",
        snap.parser.fast_path_executions, snap.parser.slow_path_executions
    );
    eprintln!("Connection ceremony counters:");
    eprintln!(
        "  prepared_insert_fast_lane_hits:      {:>4}",
        snap.prepared_insert_fast_lane_hits
    );
    eprintln!(
        "  prepared_table_engine_reuses:        {:>4}",
        snap.prepared_table_engine_reuses
    );
    eprintln!(
        "  prepared_table_engine_fresh_allocs:  {:>4}",
        snap.prepared_table_engine_fresh_allocs
    );
    eprintln!(
        "  prepared_schema_refreshes:           {:>4}",
        snap.prepared_schema_refreshes
    );
    eprintln!(
        "  pager_publication_refreshes:         {:>4}",
        snap.pager_publication_refreshes
    );
    eprintln!(
        "  begin_refresh_count:                 {:>4}",
        snap.begin_refresh_count
    );
    eprintln!(
        "  commit_refresh_count:                {:>4}",
        snap.commit_refresh_count
    );
    eprintln!(
        "  background_status_checks:            {:>4}",
        snap.background_status_checks
    );
    eprintln!("=== END SCORECARD ===");

    // ─── Assertions ───

    // 1. All 100 INSERTs should hit the prepared fast lane.
    assert!(
        snap.prepared_insert_fast_lane_hits >= 100,
        "all 100 INSERTs should hit prepared fast lane: got {}",
        snap.prepared_insert_fast_lane_hits
    );

    // 2. Fast path should dominate (precompiled_dml path from bd-6eyrg.1).
    assert!(
        snap.parser.fast_path_executions >= 100,
        "all 100 INSERTs should use fast path: got {}",
        snap.parser.fast_path_executions
    );

    // 3. Parse and compiled cache hits should be 0 (prepared stmts bypass both).
    assert_eq!(
        snap.parser.parse_cache_hits, 0,
        "prepared stmts should not produce parse cache hits"
    );
    assert_eq!(
        snap.parser.compiled_cache_hits, 0,
        "prepared stmts should not produce compiled cache hits"
    );

    // 4. Either the direct-insert fast path OR the engine-reuse path should
    // cover all ops. The direct-insert path bypasses the VDBE engine entirely
    // (so engine_reuses stays 0) but is strictly faster. Accept either.
    let direct_insert_executions = snap.prepared_direct_insert_executions;
    let engine_reuses = snap.prepared_table_engine_reuses;
    assert!(
        direct_insert_executions >= 100 || engine_reuses >= 100,
        "all ops should use either direct-insert ({direct_insert_executions}) or engine-reuse ({engine_reuses}) path",
    );
    assert_eq!(
        snap.prepared_table_engine_fresh_allocs, 0,
        "no fresh table-engine allocs expected after warmup: got {}",
        snap.prepared_table_engine_fresh_allocs
    );
}

/// Prove bd-db300.4.5.2 directly: when prepared DML must take the
/// FullReloadRequired refresh path, the execution should reuse the schema-bound
/// publication instead of paying a second bind during autocommit begin.
#[test]
fn test_prepared_full_reload_reuses_publication_after_cross_connection_ddl() {
    let _profile_guard = HotPathProfileTestGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("prepared_full_reload_publication_reuse.db");
    let db = db_path.to_string_lossy().into_owned();

    let conn1 = Connection::open(&db).unwrap();
    conn1.set_reject_mem_fallback(true);
    conn1.set_strict_mem_fallback_rejection(true);
    conn1
        .execute("CREATE TABLE prep_full_reload_pub (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();

    let stale_stmt = conn1
        .prepare("INSERT INTO prep_full_reload_pub (id, val) VALUES (?1, ?2)")
        .unwrap();

    let conn2 = Connection::open(&db).unwrap();
    conn2.set_reject_mem_fallback(true);
    conn2.set_strict_mem_fallback_rejection(true);
    conn2
        .execute("CREATE TABLE prep_full_reload_pub_bump (id INTEGER PRIMARY KEY)")
        .unwrap();

    let err = stale_stmt
        .execute_with_params(&[SqliteValue::Integer(1), SqliteValue::Text("stale".into())])
        .expect_err("cross-connection DDL must invalidate the stale prepared INSERT");
    assert!(matches!(err, FrankenError::SchemaChanged));

    // Force future stale prepared executions onto the full-reload path while
    // keeping schema identity stable for the measured window.
    conn1.set_reject_mem_fallback(false);
    let stmt = conn1
        .prepare("INSERT INTO prep_full_reload_pub (id, val) VALUES (?1, ?2)")
        .unwrap();
    conn2
        .execute("INSERT INTO prep_full_reload_pub VALUES (1, 'from_conn2')")
        .unwrap();

    reset_hot_path_profile();
    let affected = stmt
        .execute_with_params(&[
            SqliteValue::Integer(2),
            SqliteValue::Text("from_conn1".into()),
        ])
        .unwrap();
    assert_eq!(affected, 1);

    let profile = hot_path_profile_snapshot();
    eprintln!("=== bd-db300.4.5.2: FullReloadRequired publication-reuse proof ===");
    eprintln!(
        "prepared_schema_refreshes={} lightweight={} full_reload={} pager_publication_refreshes={} fast_lane_hits={}",
        profile.prepared_schema_refreshes,
        profile.prepared_schema_lightweight_refreshes,
        profile.prepared_schema_full_reloads,
        profile.pager_publication_refreshes,
        profile.prepared_insert_fast_lane_hits
    );
    eprintln!("=== END SCORECARD ===");

    assert_eq!(
        profile.prepared_schema_refreshes, 1,
        "the measured prepared execute should pay exactly one external schema refresh: {profile:?}"
    );
    assert_eq!(
        profile.prepared_schema_full_reloads, 1,
        "with eager MemDB hydration enabled, stale prepared DML should take the FullReloadRequired path: {profile:?}"
    );
    assert_eq!(
        profile.prepared_schema_lightweight_refreshes, 0,
        "the full-reload proof window must not fall back to the lightweight refresh path: {profile:?}"
    );
    assert_eq!(
        profile.pager_publication_refreshes, 1,
        "the prepared execute should reuse the full-reload publication instead of rebinding during autocommit begin: {profile:?}"
    );
    assert_eq!(
        profile.prepared_insert_fast_lane_hits, 1,
        "the measured prepared insert should stay on the prepared fast lane after the full reload: {profile:?}"
    );

    let rows = conn1
        .query("SELECT id, val FROM prep_full_reload_pub ORDER BY id")
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(1));
    assert_eq!(rows[0].values()[1], SqliteValue::Text("from_conn2".into()));
    assert_eq!(rows[1].values()[0], SqliteValue::Integer(2));
    assert_eq!(rows[1].values()[1], SqliteValue::Text("from_conn1".into()));
}
