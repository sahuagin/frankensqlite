//! Interop integrity tests: FrankenSQLite-authored databases must pass stock SQLite
//! `PRAGMA integrity_check`. Tests cover the patterns reported in issue #54 and
//! additional edge cases that exercise B-tree splitting, overflow pages, freeblock
//! accounting, and WAL checkpoint behavior.
//!
//! Every test creates a database with FrankenSQLite, closes the connection, then
//! reopens with rusqlite (stock SQLite) and asserts `PRAGMA integrity_check = "ok"`.

use tempfile::tempdir;

fn assert_stock_sqlite_integrity(db_path: &std::path::Path, label: &str) {
    let conn = rusqlite::Connection::open(db_path)
        .unwrap_or_else(|e| panic!("[{label}] stock SQLite failed to open: {e}"));
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap_or_else(|e| panic!("[{label}] integrity_check query failed: {e}"));
    assert_eq!(integrity, "ok", "[{label}] integrity_check = {integrity}");
}

/// Issue #54 exact reproduction: execute_batch with CREATE + 2 INSERTs, drop without close.
#[test]
fn issue54_execute_batch_drop_without_close() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("issue54.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute_batch(
            "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT);
             INSERT INTO items (id, label) VALUES (1, 'alpha');
             INSERT INTO items (id, label) VALUES (2, 'bravo');",
        )
        .unwrap();
        // Drop without close — matches reporter's exact pattern
    }
    assert_stock_sqlite_integrity(&path, "issue54_drop");

    let conn = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

/// Same as issue #54 but with explicit close.
#[test]
fn issue54_execute_batch_explicit_close() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("issue54_close.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute_batch(
            "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT);
             INSERT INTO items (id, label) VALUES (1, 'alpha');
             INSERT INTO items (id, label) VALUES (2, 'bravo');",
        )
        .unwrap();
        conn.close().unwrap();
    }
    assert_stock_sqlite_integrity(&path, "issue54_close");
}

/// DELETE + reinsert exercises freeblock accounting and page reuse.
#[test]
fn delete_reinsert_freeblock_accounting() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("delete_reinsert.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)").unwrap();
        conn.execute("CREATE INDEX idx_val ON t(val)").unwrap();
        for i in 0..100 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'row_{i}')")).unwrap();
        }
        conn.execute("DELETE FROM t WHERE id BETWEEN 20 AND 60").unwrap();
        for i in 200..250 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'new_{i}')")).unwrap();
        }
        // Drop without close
    }
    assert_stock_sqlite_integrity(&path, "delete_reinsert");
}

/// Delete all rows then reinsert — exercises freelist page recycling.
#[test]
fn delete_all_reinsert_freelist() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("delete_all.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)").unwrap();
        conn.execute("CREATE INDEX idx_val ON t(val)").unwrap();
        for i in 0..100 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'row_{i}')")).unwrap();
        }
        conn.execute("DELETE FROM t").unwrap();
        for i in 200..250 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'new_{i}')")).unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "delete_all_reinsert");
}

/// Multiple indexes exercise B-tree page balancing across index trees.
#[test]
fn multiple_indexes_balance() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("multi_idx.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(a INTEGER PRIMARY KEY, b TEXT, c REAL, d INTEGER)").unwrap();
        conn.execute("CREATE INDEX idx_b ON t(b)").unwrap();
        conn.execute("CREATE INDEX idx_c ON t(c)").unwrap();
        conn.execute("CREATE INDEX idx_d ON t(d)").unwrap();
        conn.execute("CREATE INDEX idx_bc ON t(b, c)").unwrap();
        for i in 0..200 {
            conn.execute(&format!(
                "INSERT INTO t VALUES ({i}, 'val_{i}', {i}.5, {})",
                i % 10
            ))
            .unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "multiple_indexes");
}

/// Large blobs trigger overflow page chains.
#[test]
fn overflow_pages() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("overflow.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, data BLOB)").unwrap();
        for i in 0..10 {
            let blob_hex: String = (0..8192)
                .map(|j| format!("{:02x}", ((i * 7 + j) % 256) as u8))
                .collect();
            conn.execute(&format!("INSERT INTO t VALUES ({i}, X'{blob_hex}')")).unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "overflow_pages");
}

/// UPDATE with indexed column exercises index entry removal + reinsertion.
#[test]
fn updates_with_index() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("updates.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)").unwrap();
        conn.execute("CREATE INDEX idx_val ON t(val)").unwrap();
        for i in 0..100 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'original_{i}')")).unwrap();
        }
        for i in (0..100).step_by(2) {
            conn.execute(&format!(
                "UPDATE t SET val = 'updated_value_longer_string_{i}' WHERE id = {i}"
            ))
            .unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "updates_with_index");
}

/// 2000 rows with indexes triggers multiple levels of B-tree splitting.
#[test]
fn large_table_btree_splitting() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("large.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, a TEXT, b TEXT, c INTEGER)").unwrap();
        conn.execute("CREATE INDEX idx_a ON t(a)").unwrap();
        conn.execute("CREATE INDEX idx_c ON t(c)").unwrap();
        for i in 0..2000 {
            conn.execute(&format!(
                "INSERT INTO t VALUES ({i}, 'name_{i}_padding_to_make_longer', \
                 'description_{i}_with_some_more_text_here', {})",
                i % 100
            ))
            .unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "large_table_2000_rows");
}

/// Composite primary key (WITHOUT ROWID-like layout for the PK index).
#[test]
fn composite_primary_key() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("composite_pk.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute(
            "CREATE TABLE edges(src INTEGER, dst INTEGER, weight REAL, PRIMARY KEY(src, dst))",
        )
        .unwrap();
        for i in 0..30 {
            for j in 0..5 {
                conn.execute(&format!("INSERT INTO edges VALUES ({i}, {j}, {i}.{j})")).unwrap();
            }
        }
    }
    assert_stock_sqlite_integrity(&path, "composite_pk");
}

/// UNIQUE constraint exercises unique index maintenance.
#[test]
fn unique_constraint_delete_reinsert() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unique.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute(
            "CREATE TABLE users(id INTEGER PRIMARY KEY, email TEXT UNIQUE, name TEXT)",
        )
        .unwrap();
        for i in 0..50 {
            conn.execute(&format!(
                "INSERT INTO users VALUES ({i}, 'user{i}@example.com', 'User {i}')"
            ))
            .unwrap();
        }
        conn.execute("DELETE FROM users WHERE id BETWEEN 20 AND 30").unwrap();
        for i in 100..111 {
            conn.execute(&format!(
                "INSERT INTO users VALUES ({i}, 'new{i}@example.com', 'New User {i}')"
            ))
            .unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "unique_constraint");
}

/// Transaction rollback then commit exercises journal/wal replay paths.
#[test]
fn transaction_rollback_then_commit() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("txn.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)").unwrap();
        for i in 0..20 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'row_{i}')")).unwrap();
        }
        conn.execute("BEGIN").unwrap();
        conn.execute("DELETE FROM t WHERE id > 10").unwrap();
        conn.execute("ROLLBACK").unwrap();
        conn.execute("BEGIN").unwrap();
        for i in 20..40 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'row_{i}')")).unwrap();
        }
        conn.execute("COMMIT").unwrap();
    }
    assert_stock_sqlite_integrity(&path, "txn_rollback_commit");
}

/// WAL journal mode exercises WAL checkpoint on close.
#[test]
fn wal_mode_checkpoint() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.db");
    {
        let conn = fsqlite::Connection::open(path.to_str().unwrap()).unwrap();
        let _ = conn.execute("PRAGMA journal_mode=WAL");
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)").unwrap();
        conn.execute("CREATE INDEX idx_val ON t(val)").unwrap();
        for i in 0..100 {
            conn.execute(&format!("INSERT INTO t VALUES ({i}, 'row_{i}')")).unwrap();
        }
    }
    assert_stock_sqlite_integrity(&path, "wal_mode");
}
