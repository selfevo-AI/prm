//! One-time, read-only import of the legacy SQLite files into an empty PostgreSQL workspace.
use crate::database::Connection;
use postgres::types::ToSql;
use std::path::Path;
pub fn import(db: &Connection, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let populated: bool =
        db.query_row("SELECT EXISTS(SELECT 1 FROM account)", &[], |r| r.get(0))?;
    if populated {
        return Ok(());
    }
    let path = directory.join("operations.sqlite3");
    if !path.exists() {
        return Ok(());
    }
    let source =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let exists: bool =
        source.query_row("SELECT EXISTS(SELECT 1 FROM account)", [], |r| r.get(0))?;
    if !exists {
        return Ok(());
    }
    let vault = rusqlite::Connection::open_with_flags(
        directory.join("mental-vault.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let payload: Vec<u8> =
        vault.query_row("SELECT payload FROM vault WHERE id=1", [], |r| r.get(0))?;
    db.execute_batch("BEGIN")?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        for (table, columns) in [
            ("account", "id,password_hash,vault_salt"),
            (
                "entities",
                "id,kind,title,worlds,value_axes,properties,status,version,created_at,updated_at",
            ),
            ("relations", "id,subject,predicate,object"),
            ("audit", "sequence,at,actor,action,target"),
        ] {
            let mut stmt = source.prepare(&format!("SELECT {columns} FROM {table}"))?;
            let count = stmt.column_count();
            let placeholders = (1..=count)
                .map(|n| format!("${n}"))
                .collect::<Vec<_>>()
                .join(",");
            let insert = format!("INSERT INTO {table}({columns}) VALUES({placeholders})");
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let mut values: Vec<Box<dyn ToSql + Sync>> = Vec::new();
                for i in 0..count {
                    let value: rusqlite::types::Value = row.get(i)?;
                    values.push(match value {
                        rusqlite::types::Value::Integer(v) => Box::new(v),
                        rusqlite::types::Value::Text(v) => Box::new(v),
                        rusqlite::types::Value::Blob(v) => Box::new(v),
                        _ => return Err("Unexpected legacy field type".into()),
                    });
                }
                let refs: Vec<&(dyn ToSql + Sync)> = values.iter().map(|v| v.as_ref()).collect();
                db.execute(&insert, &refs)?;
            }
        }
        db.execute(
            "INSERT INTO vault(id,payload) VALUES(1,$1)",
            crate::params![payload],
        )?;
        db.execute_batch("SELECT setval(pg_get_serial_sequence('audit','sequence'), COALESCE((SELECT MAX(sequence) FROM audit),1), EXISTS(SELECT 1 FROM audit));")?;
        db.execute("INSERT INTO audit(at,actor,action,target) VALUES($1,'owner','database.migrated','postgresql')",crate::params![crate::store::now()])?;
        db.execute_batch("COMMIT")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = db.execute_batch("ROLLBACK");
    }
    result
}
