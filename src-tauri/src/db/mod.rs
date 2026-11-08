use std::path::Path;

use rusqlite::Connection;
use uuid::Uuid;

use crate::error::Result;

// Para novas migrações, basta acrescentar o próximo arquivo aqui.
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/001_init.sql"),
    include_str!("../../migrations/002_relations_origin.sql"),
];

/// Versão do esquema que este app conhece (usada para recusar backups de versões mais novas).
pub const SCHEMA_VERSION: usize = MIGRATIONS.len();

/// UUID v7: único e ordenável por data de criação.
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let current: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}