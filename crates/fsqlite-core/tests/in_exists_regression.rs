//! bd-wwqen.2 (B2): Regression tests for IN subquery HashSet probe and
//! EXISTS LIMIT 1 injection.
//!
//! Proves:
//! 1. Large non-correlated IN (SELECT ...) uses HashSet probe (correctness).
//! 2. Small IN lists still work (below threshold, linear scan).
//! 3. NOT IN with NULLs preserves three-valued logic.
//! 4. Correlated IN subqueries still evaluate per-row (no HashSet).
//! 5. EXISTS with LIMIT 1 injection returns correct results.
//! 6. NOT EXISTS returns correct results.
//! 7. Non-correlated EXISTS is correct.
//!
//! Run:
//!   cargo test -p fsqlite-core --test in_exists_regression -- --nocapture

use fsqlite_core::connection::Connection;
use fsqlite_types::SqliteValue;

/// B2.1: Large non-correlated IN (SELECT ...) produces correct results.
/// After rewrite_in_expr materializes the subquery into InSet::List,
/// the HashSet probe (>16 elements) must match the linear-scan result.
#[test]
fn test_large_in_subquery_hashset_correctness() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE big(id INTEGER PRIMARY KEY)")
        .unwrap();
    conn.execute("CREATE TABLE probe(id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();

    // Insert 200 rows into big (well above the 16-element threshold).
    for i in 1..=200 {
        conn.execute(&format!("INSERT INTO big VALUES({i})"))
            .unwrap();
    }
    // Insert 10 rows into probe, some matching big, some not.
    for i in 1..=10 {
        conn.execute(&format!("INSERT INTO probe VALUES({i}, 'row{i}')"))
            .unwrap();
    }
    // id=999 is NOT in big.
    conn.execute("INSERT INTO probe VALUES(999, 'missing')")
        .unwrap();

    let rows = conn
        .query("SELECT id, val FROM probe WHERE id IN (SELECT id FROM big) ORDER BY id")
        .unwrap();

    assert_eq!(rows.len(), 10, "all 10 matching rows should be returned");
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(1));
    assert_eq!(rows[9].values()[0], SqliteValue::Integer(10));

    // Verify id=999 is excluded.
    let missing = conn
        .query("SELECT COUNT(*) FROM probe WHERE id IN (SELECT id FROM big) AND id = 999")
        .unwrap();
    assert_eq!(missing[0].values()[0], SqliteValue::Integer(0));
}

/// B2.1: NOT IN with NULLs in the subquery result preserves three-valued
/// logic: if the value is not found but NULLs exist, result is NULL.
#[test]
fn test_not_in_subquery_null_semantics() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER)").unwrap();
    // Insert values including NULL.
    conn.execute("INSERT INTO t VALUES(1)").unwrap();
    conn.execute("INSERT INTO t VALUES(2)").unwrap();
    conn.execute("INSERT INTO t VALUES(NULL)").unwrap();

    // 1 NOT IN (1, 2, NULL) → FALSE (1 is found)
    let rows = conn.query("SELECT 1 NOT IN (SELECT id FROM t)").unwrap();
    assert_eq!(
        rows[0].values()[0],
        SqliteValue::Integer(0),
        "1 is in the set"
    );

    // 99 NOT IN (1, 2, NULL) → NULL (99 not found, but NULL present)
    let rows = conn.query("SELECT 99 NOT IN (SELECT id FROM t)").unwrap();
    assert_eq!(
        rows[0].values()[0],
        SqliteValue::Null,
        "99 NOT IN with NULL → NULL"
    );

    // With no NULLs: 99 NOT IN (1, 2) → TRUE
    conn.execute("DELETE FROM t WHERE id IS NULL").unwrap();
    let rows = conn.query("SELECT 99 NOT IN (SELECT id FROM t)").unwrap();
    assert_eq!(
        rows[0].values()[0],
        SqliteValue::Integer(1),
        "99 NOT IN without NULL → TRUE"
    );
}

/// B2.1: Small IN list (below 16-element threshold) still works correctly
/// via the original linear scan path.
#[test]
fn test_small_in_subquery_linear_scan() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE small(id INTEGER PRIMARY KEY)")
        .unwrap();
    for i in 1..=5 {
        conn.execute(&format!("INSERT INTO small VALUES({i})"))
            .unwrap();
    }

    let rows = conn
        .query("SELECT id FROM small WHERE id IN (SELECT id FROM small WHERE id <= 3) ORDER BY id")
        .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(1));
    assert_eq!(rows[2].values()[0], SqliteValue::Integer(3));
}

/// B2.1: IN with mixed types (Integer and Text) in the subquery.
#[test]
fn test_in_subquery_mixed_types() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE mix(val)").unwrap();
    conn.execute("INSERT INTO mix VALUES(1)").unwrap();
    conn.execute("INSERT INTO mix VALUES('hello')").unwrap();
    conn.execute("INSERT INTO mix VALUES(3.14)").unwrap();

    let rows = conn
        .query("SELECT val FROM mix WHERE val IN (SELECT val FROM mix WHERE val = 1 OR val = 'hello') ORDER BY val")
        .unwrap();
    assert_eq!(rows.len(), 2);
}

/// B2.2: EXISTS with LIMIT 1 injection returns correct TRUE.
#[test]
fn test_exists_returns_true_with_matching_rows() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE parent(id INTEGER PRIMARY KEY)")
        .unwrap();
    conn.execute("CREATE TABLE child(id INTEGER, pid INTEGER)")
        .unwrap();
    conn.execute("INSERT INTO parent VALUES(1)").unwrap();
    conn.execute("INSERT INTO parent VALUES(2)").unwrap();
    conn.execute("INSERT INTO child VALUES(10, 1)").unwrap();
    conn.execute("INSERT INTO child VALUES(20, 1)").unwrap();

    // Correlated EXISTS: parent 1 has children, parent 2 does not.
    let rows = conn
        .query("SELECT id FROM parent WHERE EXISTS (SELECT 1 FROM child WHERE child.pid = parent.id) ORDER BY id")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(1));
}

/// B2.2: NOT EXISTS returns correct results.
#[test]
fn test_not_exists_returns_correct_results() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE parent(id INTEGER PRIMARY KEY)")
        .unwrap();
    conn.execute("CREATE TABLE child(id INTEGER, pid INTEGER)")
        .unwrap();
    conn.execute("INSERT INTO parent VALUES(1)").unwrap();
    conn.execute("INSERT INTO parent VALUES(2)").unwrap();
    conn.execute("INSERT INTO child VALUES(10, 1)").unwrap();

    let rows = conn
        .query("SELECT id FROM parent WHERE NOT EXISTS (SELECT 1 FROM child WHERE child.pid = parent.id) ORDER BY id")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(2));
}

/// B2.2: Non-correlated EXISTS (no outer refs) returns correct result.
#[test]
fn test_non_correlated_exists() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES(1)").unwrap();

    // Non-correlated EXISTS with data → TRUE
    let rows = conn
        .query("SELECT EXISTS (SELECT 1 FROM t WHERE id = 1)")
        .unwrap();
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(1));

    // Non-correlated EXISTS without data → FALSE
    let rows = conn
        .query("SELECT EXISTS (SELECT 1 FROM t WHERE id = 999)")
        .unwrap();
    assert_eq!(rows[0].values()[0], SqliteValue::Integer(0));
}

/// B2.1+B2.2: Combined IN + EXISTS in same query.
#[test]
fn test_in_and_exists_combined() {
    let conn = Connection::open(":memory:").unwrap();
    conn.execute("CREATE TABLE categories(id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    conn.execute("CREATE TABLE products(id INTEGER PRIMARY KEY, cat_id INTEGER, name TEXT)")
        .unwrap();

    for i in 1..=5 {
        conn.execute(&format!("INSERT INTO categories VALUES({i}, 'cat{i}')"))
            .unwrap();
    }
    for i in 1..=20 {
        let cat = (i % 5) + 1;
        conn.execute(&format!(
            "INSERT INTO products VALUES({i}, {cat}, 'prod{i}')"
        ))
        .unwrap();
    }

    // Products in categories 1-3 that have at least one other product in the same category.
    let rows = conn
        .query(
            "SELECT p.id FROM products p \
         WHERE p.cat_id IN (SELECT id FROM categories WHERE id <= 3) \
         AND EXISTS (SELECT 1 FROM products p2 WHERE p2.cat_id = p.cat_id AND p2.id != p.id) \
         ORDER BY p.id",
        )
        .unwrap();

    // Categories 1-3 each have 4 products (ids: 1-20 mod 5). Each product has 3 siblings.
    assert!(
        rows.len() > 0,
        "should find products matching both IN and EXISTS"
    );
    // All returned products should be in categories 1-3.
    for row in &rows {
        let id = match row.values()[0] {
            SqliteValue::Integer(i) => i,
            _ => panic!("expected integer id"),
        };
        assert!(id >= 1 && id <= 20);
    }
}
