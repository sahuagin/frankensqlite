use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};

use fsqlite_error::{FrankenError, Result};
use fsqlite_types::DatabaseHeader;
use fsqlite_types::cx::Cx;
use fsqlite_vdbe::codegen::TableSchema;
use fsqlite_vdbe::engine::MemDatabase;
#[cfg(not(target_arch = "wasm32"))]
use fsqlite_vfs::host_fs;

use crate::compat_persist::SqliteMasterEntry;
#[cfg(not(target_arch = "wasm32"))]
use crate::compat_persist::persist_to_sqlite_with_header_and_master_entries;

pub(crate) const ATTACHED_SCHEMA_UNSUPPORTED: &str = "VACUUM on attached schemas";

#[cfg(not(target_arch = "wasm32"))]
static NEXT_TEMP_REBUILD_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn persist_compacted_database(
    cx: &Cx,
    target_path: &Path,
    schema: &[TableSchema],
    db: &MemDatabase,
    header: &DatabaseHeader,
    extra_master_entries: &[SqliteMasterEntry],
) -> Result<()> {
    persist_to_sqlite_with_header_and_master_entries(
        cx,
        target_path,
        schema,
        db,
        header,
        extra_master_entries,
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn persist_compacted_database(
    _cx: &Cx,
    _target_path: &Path,
    _schema: &[TableSchema],
    _db: &MemDatabase,
    _header: &DatabaseHeader,
    _extra_master_entries: &[SqliteMasterEntry],
) -> Result<()> {
    Err(FrankenError::not_implemented(
        "VACUUM is not supported on wasm32",
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn validate_vacuum_into_target(source_path: &str, target_path: &Path) -> Result<()> {
    if target_path.as_os_str().is_empty() {
        return Err(FrankenError::CannotOpen {
            path: target_path.to_path_buf(),
        });
    }
    if target_path == Path::new(source_path) || host_fs::metadata(target_path).is_ok() {
        return Err(FrankenError::CannotOpen {
            path: target_path.to_path_buf(),
        });
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn validate_vacuum_into_target(_source_path: &str, target_path: &Path) -> Result<()> {
    if target_path.as_os_str().is_empty() {
        return Err(FrankenError::CannotOpen {
            path: target_path.to_path_buf(),
        });
    }
    Err(FrankenError::not_implemented(
        "VACUUM is not supported on wasm32",
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn temp_rebuild_path(source_path: &Path) -> PathBuf {
    let seq = NEXT_TEMP_REBUILD_ID.fetch_add(1, Ordering::Relaxed);
    let mut name = source_path
        .file_name()
        .map_or_else(|| "main".into(), std::ffi::OsString::from);
    name.push(format!(".fsqlite-vacuum-{seq}.tmp"));
    source_path.with_file_name(name)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn temp_rebuild_path(source_path: &Path) -> PathBuf {
    source_path.to_path_buf()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn replace_database_file(target_path: &Path, rebuilt_path: &Path) -> Result<()> {
    let bytes = host_fs::read(rebuilt_path)?;
    host_fs::write(target_path, bytes)?;
    drop(host_fs::remove_file(rebuilt_path));
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn replace_database_file(_target_path: &Path, _rebuilt_path: &Path) -> Result<()> {
    Err(FrankenError::not_implemented(
        "VACUUM is not supported on wasm32",
    ))
}

#[cfg(test)]
mod tests {
    use crate::connection::Connection;

    #[test]
    fn test_vacuum_rebuilds_file_backed_database_and_preserves_header_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("vacuum-in-place.db");
        let db = db_path.to_string_lossy().into_owned();

        let conn = Connection::open_with_page_size(&db, 1024).unwrap();
        conn.execute("PRAGMA user_version = 321;").unwrap();
        conn.execute("PRAGMA application_id = 654321;").unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, payload TEXT);")
            .unwrap();

        let mut insert_sql = String::from("BEGIN;");
        for rowid in 1..=120_u32 {
            insert_sql.push_str(&format!(
                "INSERT INTO t(id, payload) VALUES ({rowid}, '{}');",
                "x".repeat(700)
            ));
        }
        insert_sql.push_str("COMMIT;");
        conn.execute_batch(&insert_sql).unwrap();
        conn.execute("DELETE FROM t WHERE id <= 100;").unwrap();
        drop(conn);

        let oracle_before = rusqlite::Connection::open(&db_path).unwrap();
        let freelist_before: i64 = oracle_before
            .query_row("PRAGMA freelist_count;", [], |row| row.get(0))
            .unwrap();
        assert!(
            freelist_before > 0,
            "expected deletions to create free pages before VACUUM"
        );
        drop(oracle_before);

        let conn = Connection::open(&db).unwrap();
        conn.execute("VACUUM;").unwrap();
        let rows = conn
            .query("SELECT COUNT(*), MIN(id), MAX(id) FROM t;")
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].values()[0],
            fsqlite_types::value::SqliteValue::Integer(20)
        );
        assert_eq!(
            rows[0].values()[1],
            fsqlite_types::value::SqliteValue::Integer(101)
        );
        assert_eq!(
            rows[0].values()[2],
            fsqlite_types::value::SqliteValue::Integer(120)
        );
        drop(conn);

        let oracle_after = rusqlite::Connection::open(&db_path).unwrap();
        let page_size: i64 = oracle_after
            .query_row("PRAGMA page_size;", [], |row| row.get(0))
            .unwrap();
        let user_version: i64 = oracle_after
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        let application_id: i64 = oracle_after
            .query_row("PRAGMA application_id;", [], |row| row.get(0))
            .unwrap();
        let freelist_after: i64 = oracle_after
            .query_row("PRAGMA freelist_count;", [], |row| row.get(0))
            .unwrap();
        let row_count: i64 = oracle_after
            .query_row("SELECT COUNT(*) FROM t;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(page_size, 1024);
        assert_eq!(user_version, 321);
        assert_eq!(application_id, 654321);
        assert_eq!(freelist_after, 0);
        assert_eq!(row_count, 20);
    }

    #[test]
    fn test_vacuum_into_writes_compacted_copy_with_preserved_page_size_and_pragmas() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("vacuum-into-source.db");
        let target_path = dir.path().join("vacuum-into-target.db");
        let source = source_path.to_string_lossy().into_owned();
        let target = target_path.to_string_lossy().into_owned();

        let conn = Connection::open_with_page_size(&source, 8192).unwrap();
        conn.execute("PRAGMA user_version = 777;").unwrap();
        conn.execute("PRAGMA application_id = 888;").unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, payload TEXT);")
            .unwrap();
        conn.execute("INSERT INTO t VALUES (1, 'alpha'), (2, 'beta'), (3, 'gamma');")
            .unwrap();
        conn.execute("DELETE FROM t WHERE id = 2;").unwrap();
        conn.execute_with_params(
            "VACUUM INTO ?1;",
            &[fsqlite_types::value::SqliteValue::Text(
                target.clone().into(),
            )],
        )
        .unwrap();
        drop(conn);

        let copied = rusqlite::Connection::open(&target_path).unwrap();
        let page_size: i64 = copied
            .query_row("PRAGMA page_size;", [], |row| row.get(0))
            .unwrap();
        let user_version: i64 = copied
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        let application_id: i64 = copied
            .query_row("PRAGMA application_id;", [], |row| row.get(0))
            .unwrap();
        let freelist_count: i64 = copied
            .query_row("PRAGMA freelist_count;", [], |row| row.get(0))
            .unwrap();
        let values: Vec<(i64, String)> = {
            let mut stmt = copied
                .prepare("SELECT id, payload FROM t ORDER BY id;")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };

        assert_eq!(page_size, 8192);
        assert_eq!(user_version, 777);
        assert_eq!(application_id, 888);
        assert_eq!(freelist_count, 0);
        assert_eq!(
            values,
            vec![(1, "alpha".to_owned()), (3, "gamma".to_owned())]
        );
    }

    #[test]
    fn test_vacuum_in_place_removes_rebuild_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("vacuum-temp-cleanup.db");
        let db = db_path.to_string_lossy().into_owned();

        let conn = Connection::open(&db).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, payload TEXT);")
            .unwrap();
        conn.execute("INSERT INTO t VALUES (1, 'alpha'), (2, 'beta');")
            .unwrap();

        conn.execute("VACUUM;").unwrap();

        let temp_prefix = format!(
            "{}.fsqlite-vacuum-",
            db_path.file_name().unwrap().to_string_lossy()
        );
        let temp_files: Vec<_> = fsqlite_vfs::host_fs::read_dir_paths(dir.path())
            .unwrap()
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy())
                    .is_some_and(|name| name.starts_with(&temp_prefix) && name.ends_with(".tmp"))
            })
            .collect();
        assert!(
            temp_files.is_empty(),
            "VACUUM should not leave rebuild temp files behind: {temp_files:?}"
        );
    }

    #[test]
    fn test_vacuum_preserves_views_and_triggers_across_rebuild_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("vacuum-schema-objects.db");
        let db = db_path.to_string_lossy().into_owned();

        let conn = Connection::open(&db).unwrap();
        conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, payload TEXT);")
            .unwrap();
        conn.execute("CREATE TABLE audit(id INTEGER PRIMARY KEY);")
            .unwrap();
        conn.execute("CREATE VIEW live_t AS SELECT id, payload FROM t WHERE id > 0;")
            .unwrap();
        conn.execute(
            "CREATE TRIGGER t_audit AFTER INSERT ON t BEGIN INSERT INTO audit(id) VALUES (NEW.id); END;",
        )
        .unwrap();
        conn.execute("INSERT INTO t(id, payload) VALUES (1, 'alpha');")
            .unwrap();

        conn.execute("VACUUM;").unwrap();

        let live_rows = conn
            .query("SELECT id, payload FROM live_t ORDER BY id;")
            .unwrap();
        assert_eq!(live_rows.len(), 1);
        assert_eq!(
            live_rows[0].values()[0],
            fsqlite_types::value::SqliteValue::Integer(1)
        );
        assert_eq!(
            live_rows[0].values()[1],
            fsqlite_types::value::SqliteValue::Text("alpha".into())
        );

        conn.execute("INSERT INTO t(id, payload) VALUES (2, 'beta');")
            .unwrap();
        let audit_rows = conn.query("SELECT id FROM audit ORDER BY id;").unwrap();
        assert_eq!(audit_rows.len(), 2);
        assert_eq!(
            audit_rows[0].values()[0],
            fsqlite_types::value::SqliteValue::Integer(1)
        );
        assert_eq!(
            audit_rows[1].values()[0],
            fsqlite_types::value::SqliteValue::Integer(2)
        );
        drop(conn);

        let sqlite = rusqlite::Connection::open(&db_path).unwrap();
        let schema_rows: Vec<(String, String)> = {
            let mut stmt = sqlite
                .prepare(
                    "SELECT type, name
                     FROM sqlite_master
                     WHERE name IN ('live_t', 't_audit')
                     ORDER BY type, name;",
                )
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert_eq!(
            schema_rows,
            vec![
                ("trigger".to_owned(), "t_audit".to_owned()),
                ("view".to_owned(), "live_t".to_owned()),
            ]
        );

        sqlite
            .execute("INSERT INTO t(id, payload) VALUES (3, 'gamma');", [])
            .unwrap();
        let audit_ids: Vec<i64> = {
            let mut stmt = sqlite.prepare("SELECT id FROM audit ORDER BY id;").unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert_eq!(audit_ids, vec![1, 2, 3]);
    }
}
