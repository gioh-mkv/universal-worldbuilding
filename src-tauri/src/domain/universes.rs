use rusqlite::{params, Connection};
use serde::Serialize;

use crate::db::new_id;
use crate::error::{AppError, Result};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Universe {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

fn clean_description(d: Option<&str>) -> Option<String> {
    d.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

pub fn create(conn: &Connection, name: &str, description: Option<&str>) -> Result<Universe> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::msg("o universo precisa de um nome"));
    }
    let description = clean_description(description);
    let id = new_id();
    conn.execute(
        "INSERT INTO universes (id, name, description) VALUES (?1, ?2, ?3)",
        params![id, name, description],
    )?;
    Ok(Universe {
        id,
        name: name.to_string(),
        description,
    })
}

pub fn update(
    conn: &Connection,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<Universe> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::msg("o universo precisa de um nome"));
    }
    let description = clean_description(description);
    let changed = conn.execute(
        "UPDATE universes SET name = ?1, description = ?2, updated_at = datetime('now')
         WHERE id = ?3",
        params![name, description, id],
    )?;
    if changed == 0 {
        return Err(AppError::msg("universo não encontrado"));
    }
    Ok(Universe {
        id: id.to_string(),
        name: name.to_string(),
        description,
    })
}

/// Apaga o universo e TUDO dentro dele (tipos, entidades, relações, documentos).
/// A ordem explícita evita depender da ordem das cascatas de chaves estrangeiras.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM universes WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Err(AppError::msg("universo não encontrado"));
    }
    tx.execute("DELETE FROM relations WHERE universe_id = ?1", [id])?;
    tx.execute("DELETE FROM search_index WHERE universe_id = ?1", [id])?;
    tx.execute("DELETE FROM entities WHERE universe_id = ?1", [id])?; // cascata: menções e versões
    tx.execute("DELETE FROM universes WHERE id = ?1", [id])?; // cascata: tipos, propriedades, tipos de relação
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<Universe>> {
    let mut st = conn.prepare("SELECT id, name, description FROM universes ORDER BY name")?;
    let rows = st.query_map([], |r| {
        Ok(Universe {
            id: r.get(0)?,
            name: r.get(1)?,
            description: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{entities, entity_types, relations};
    use serde_json::json;
    use std::path::Path;

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn excluir_universo_apaga_tudo_dele_e_preserva_os_outros() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let a = create(&conn, "A", None).unwrap();
        let b = create(&conn, "B", None).unwrap();

        for u in [&a, &b] {
            let t = entity_types::create(
                &conn,
                serde_json::from_value(json!({
                    "universeId": u.id, "name": "Tipo",
                    "properties": [{"key": "x", "label": "X", "dataType": "text"}]
                }))
                .unwrap(),
            )
            .unwrap();
            let mk = |name: &str| {
                entities::create(
                    &conn,
                    serde_json::from_value(json!({
                        "universeId": u.id, "typeId": t.id, "name": name
                    }))
                    .unwrap(),
                )
                .unwrap()
            };
            let (e1, e2) = (mk("E1"), mk("E2"));
            let rt = relations::create_type(
                &conn,
                serde_json::from_value(json!({
                    "universeId": u.id, "name": "conhece", "inverseName": "é conhecido por"
                }))
                .unwrap(),
            )
            .unwrap();
            relations::create(
                &conn,
                serde_json::from_value(json!({
                    "universeId": u.id, "typeId": rt.id, "sourceId": e1.id, "targetId": e2.id
                }))
                .unwrap(),
            )
            .unwrap();
        }

        delete(&conn, &a.id).unwrap();

        assert_eq!(list(&conn).unwrap().len(), 1);
        assert_eq!(count(&conn, "entities"), 2);
        assert_eq!(count(&conn, "entity_types"), 1);
        assert_eq!(count(&conn, "property_defs"), 1);
        assert_eq!(count(&conn, "relation_types"), 1);
        assert_eq!(count(&conn, "relations"), 1);
        assert_eq!(count(&conn, "search_index"), 2);
    }
}