use rusqlite::Connection;
use serde::Serialize;

use crate::error::Result;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRef {
    pub id: String,
    pub name: String,
    pub type_id: String,
}

#[derive(Serialize)]
pub struct EntityLinks {
    /// Entidades que este documento menciona.
    pub outgoing: Vec<EntityRef>,
    /// Entidades cujos documentos mencionam esta (backlinks).
    pub incoming: Vec<EntityRef>,
}

fn refs(conn: &Connection, sql: &str, id: &str) -> Result<Vec<EntityRef>> {
    let mut st = conn.prepare(sql)?;
    let rows = st.query_map([id], |r| {
        Ok(EntityRef {
            id: r.get(0)?,
            name: r.get(1)?,
            type_id: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn for_entity(conn: &Connection, id: &str) -> Result<EntityLinks> {
    let outgoing = refs(
        conn,
        "SELECT e.id, e.name, e.type_id
         FROM mentions m JOIN entities e ON e.id = m.target_id
         WHERE m.source_id = ?1 ORDER BY e.name COLLATE NOCASE",
        id,
    )?;
    let incoming = refs(
        conn,
        "SELECT e.id, e.name, e.type_id
         FROM mentions m JOIN entities e ON e.id = m.source_id
         WHERE m.target_id = ?1 ORDER BY e.name COLLATE NOCASE",
        id,
    )?;
    Ok(EntityLinks { outgoing, incoming })
}