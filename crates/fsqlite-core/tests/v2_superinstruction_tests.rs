//! V2 superinstruction fusion + ceremony reduction regression tests.
//!
//! Verifies correctness after V2.1 (FusedAppendInsert), V2.2
//! (FusedOpenWriteLast, currently disabled), and V2.3 (bounds check +
//! metrics gating) optimizations.

use fsqlite_core::connection::Connection;
use fsqlite_types::SqliteValue;

fn new_mem_conn() -> Connection {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("PRAGMA journal_mode=WAL").ok();
    conn
}

// ── V2.1: FusedAppendInsert correctness ─────────────────────────────────

#[test]
fn test_v2_fused_insert_simple_1col() {
    // Simple INSERT that should trigger the FusedAppendInsert peephole
    // (NewRowid + MakeRecord + Insert with ABORT conflict, no indexes)
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=1000 {
        conn.execute(&format!("INSERT INTO t VALUES ({i})"))
            .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 1000);

    let max = conn.query_row("SELECT MAX(id) FROM t").unwrap();
    assert_eq!(max.values()[0].to_integer(), 1000);

    let min = conn.query_row("SELECT MIN(id) FROM t").unwrap();
    assert_eq!(min.values()[0].to_integer(), 1);
}

#[test]
fn test_v2_fused_insert_multicol_varied_types() {
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score REAL, data BLOB)")
        .unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=500 {
        conn.execute(&format!(
            "INSERT INTO t VALUES ({i}, 'user_{i}', {}.5, X'DEADBEEF')",
            i
        ))
        .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 500);

    let row = conn
        .query_row("SELECT name, score FROM t WHERE id = 250")
        .unwrap();
    assert_eq!(row.values()[0].to_text(), "user_250");
    assert!((row.values()[1].to_float() - 250.5).abs() < 0.001);
}

#[test]
fn test_v2_fused_insert_autoincrement() {
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, val TEXT)")
        .unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=100 {
        conn.execute(&format!("INSERT INTO t (val) VALUES ('item_{i}')"))
            .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 100);

    // Verify monotonically increasing rowids
    let max = conn.query_row("SELECT MAX(id) FROM t").unwrap();
    assert!(max.values()[0].to_integer() >= 100);
}

#[test]
fn test_v2_fused_insert_empty_table_first_row() {
    // First INSERT into empty table — no cached last_alloc_rowid
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 'first')").unwrap();

    let row = conn.query_row("SELECT id, val FROM t").unwrap();
    assert_eq!(row.values()[0].to_integer(), 1);
    assert_eq!(row.values()[1].to_text(), "first");
}

#[test]
fn test_v2_fused_insert_after_delete() {
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, val TEXT)")
        .unwrap();

    // Insert some rows
    for i in 1..=10 {
        conn.execute(&format!("INSERT INTO t (val) VALUES ('item_{i}')"))
            .unwrap();
    }
    // Delete some
    conn.execute("DELETE FROM t WHERE id > 5").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 5);

    // Insert more — AUTOINCREMENT should not reuse deleted rowids
    for i in 11..=15 {
        conn.execute(&format!("INSERT INTO t (val) VALUES ('new_{i}')"))
            .unwrap();
    }

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 10);

    let min_new = conn
        .query_row("SELECT MIN(id) FROM t WHERE val LIKE 'new_%'")
        .unwrap();
    assert!(
        min_new.values()[0].to_integer() > 10,
        "AUTOINCREMENT should not reuse deleted rowids"
    );
}

#[test]
fn test_v2_insert_not_fused_with_index() {
    // Table with secondary index — should NOT use FusedAppendInsert
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT UNIQUE)")
        .unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=100 {
        conn.execute(&format!("INSERT INTO t VALUES ({i}, 'unique_{i}')"))
            .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 100);

    // Verify UNIQUE constraint works
    let result = conn.execute("INSERT INTO t VALUES (101, 'unique_1')");
    assert!(
        result.is_err(),
        "UNIQUE constraint should prevent duplicate"
    );
}

#[test]
fn test_v2_insert_not_fused_with_conflict() {
    // INSERT OR REPLACE — should NOT use FusedAppendInsert
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 'original')")
        .unwrap();
    conn.execute("INSERT OR REPLACE INTO t VALUES (1, 'replaced')")
        .unwrap();

    let row = conn.query_row("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(row.values()[0].to_text(), "replaced");
}

// ── V2.3: Ceremony reduction correctness ────────────────────────────────

#[test]
fn test_v2_halt_sentinel_terminates() {
    // Programs must terminate via Halt — verify no infinite loop
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1)").unwrap();
    let row = conn.query_row("SELECT * FROM t").unwrap();
    assert_eq!(row.values()[0].to_integer(), 1);
}

#[test]
fn test_v2_prepared_insert_matches_adhoc() {
    // Verify prepared INSERT produces same results as ad-hoc INSERT
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t1 (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    conn.execute("CREATE TABLE t2 (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();

    // Ad-hoc inserts into t1
    conn.execute("BEGIN").unwrap();
    for i in 1..=100 {
        conn.execute(&format!("INSERT INTO t1 VALUES ({i}, {})", i * 10))
            .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    // Prepared inserts into t2
    let stmt = conn.prepare("INSERT INTO t2 VALUES (?1, ?2)").unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=100_i64 {
        stmt.execute_with_params(&[SqliteValue::Integer(i), SqliteValue::Integer(i * 10)])
            .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    // Compare results
    let sum1 = conn.query_row("SELECT SUM(val) FROM t1").unwrap();
    let sum2 = conn.query_row("SELECT SUM(val) FROM t2").unwrap();
    assert_eq!(sum1.values()[0].to_integer(), sum2.values()[0].to_integer());

    let count1 = conn.query_row("SELECT COUNT(*) FROM t1").unwrap();
    let count2 = conn.query_row("SELECT COUNT(*) FROM t2").unwrap();
    assert_eq!(
        count1.values()[0].to_integer(),
        count2.values()[0].to_integer()
    );
}

// ── Mixed fused and normal operations ───────────────────────────────────

#[test]
fn test_v2_mixed_insert_update_in_transaction() {
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();

    conn.execute("BEGIN").unwrap();
    // Fused INSERT path (no index, ABORT mode)
    for i in 1..=50 {
        conn.execute(&format!("INSERT INTO t VALUES ({i}, {i})"))
            .unwrap();
    }
    // UPDATE (not fused — different opcode path)
    conn.execute("UPDATE t SET val = val * 2 WHERE id <= 25")
        .unwrap();
    // More INSERTs
    for i in 51..=100 {
        conn.execute(&format!("INSERT INTO t VALUES ({i}, {i})"))
            .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 100);

    // Verify UPDATE applied
    let row = conn.query_row("SELECT val FROM t WHERE id = 10").unwrap();
    assert_eq!(row.values()[0].to_integer(), 20); // 10 * 2
    let row = conn.query_row("SELECT val FROM t WHERE id = 30").unwrap();
    assert_eq!(row.values()[0].to_integer(), 30); // not doubled
}

#[test]
fn test_v2_large_insert_page_splits() {
    // Enough data to trigger multiple B-tree page splits during fused insert
    let conn = new_mem_conn();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, data TEXT)")
        .unwrap();

    conn.execute("BEGIN").unwrap();
    for i in 1..=5000 {
        conn.execute(&format!(
            "INSERT INTO t VALUES ({i}, 'data_padding_{i}_xxxxxxxxxxxxxxxxxx')"
        ))
        .unwrap();
    }
    conn.execute("COMMIT").unwrap();

    let count = conn.query_row("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count.values()[0].to_integer(), 5000);

    // Verify B-tree integrity via spot checks
    let first = conn.query_row("SELECT data FROM t WHERE id = 1").unwrap();
    assert!(first.values()[0].to_text().starts_with("data_padding_1_"));
    let last = conn
        .query_row("SELECT data FROM t WHERE id = 5000")
        .unwrap();
    assert!(last.values()[0].to_text().starts_with("data_padding_5000_"));
}
