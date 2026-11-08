use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::links::EntityRef;
use crate::db::new_id;
use crate::error::{AppError, Result};

const DIRECTIONS: [&str; 3] = ["directed", "bidirectional", "hierarchical"];

fn directed() -> String {
    "directed".to_string()
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
}

// ---------- Tipos de relação ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RelationType {
    pub id: String,
    pub universe_id: String,
    pub name: String,
    pub inverse_name: Option<String>,
    pub directionality: String,
    /// Quantas relações usam este tipo.
    pub usage_count: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRelationType {
    pub universe_id: String,
    pub name: String,
    pub inverse_name: Option<String>,
    #[serde(default = "directed")]
    pub directionality: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRelationType {
    pub id: String,
    pub name: String,
    pub inverse_name: Option<String>,
    pub directionality: String,
}

const TYPE_COLS: &str = "id, universe_id, name, inverse_name, directionality,
    (SELECT COUNT(*) FROM relations r WHERE r.type_id = relation_types.id)";

fn type_from_row(r: &rusqlite::Row) -> rusqlite::Result<RelationType> {
    Ok(RelationType {
        id: r.get(0)?,
        universe_id: r.get(1)?,
        name: r.get(2)?,
        inverse_name: r.get(3)?,
        directionality: r.get(4)?,
        usage_count: r.get(5)?,
    })
}

pub fn get_type(conn: &Connection, id: &str) -> Result<RelationType> {
    conn.query_row(
        &format!("SELECT {TYPE_COLS} FROM relation_types WHERE id = ?1"),
        [id],
        type_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::msg("tipo de relação não encontrado"))
}

pub fn list_types(conn: &Connection, universe_id: &str) -> Result<Vec<RelationType>> {
    let mut st = conn.prepare(&format!(
        "SELECT {TYPE_COLS} FROM relation_types
         WHERE universe_id = ?1 ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = st.query_map([universe_id], type_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn ensure_unique_type_name(
    conn: &Connection,
    universe_id: &str,
    name: &str,
    except_id: Option<&str>,
) -> Result<()> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM relation_types
         WHERE universe_id = ?1 AND name = ?2 COLLATE NOCASE AND (?3 IS NULL OR id <> ?3)",
        params![universe_id, name, except_id],
        |r| r.get(0),
    )?;
    if n > 0 {
        return Err(AppError::msg("já existe um tipo de relação com esse nome"));
    }
    Ok(())
}

pub fn create_type(conn: &Connection, input: NewRelationType) -> Result<RelationType> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::msg("o tipo de relação precisa de um nome"));
    }
    if !DIRECTIONS.contains(&input.directionality.as_str()) {
        return Err(AppError::msg("direcionalidade inválida"));
    }
    ensure_unique_type_name(conn, &input.universe_id, name, None)?;
    // Relações bidirecionais usam o mesmo nome nos dois lados.
    let inverse = if input.directionality == "bidirectional" {
        None
    } else {
        clean(&input.inverse_name)
    };
    let id = new_id();
    conn.execute(
        "INSERT INTO relation_types (id, universe_id, name, inverse_name, directionality)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, input.universe_id, name, inverse, input.directionality],
    )?;
    get_type(conn, &id)
}

pub fn update_type(conn: &Connection, input: UpdateRelationType) -> Result<RelationType> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::msg("o tipo de relação precisa de um nome"));
    }
    if !DIRECTIONS.contains(&input.directionality.as_str()) {
        return Err(AppError::msg("direcionalidade inválida"));
    }
    let current = get_type(conn, &input.id)?;
    ensure_unique_type_name(conn, &current.universe_id, name, Some(&current.id))?;
    let inverse = if input.directionality == "bidirectional" {
        None
    } else {
        clean(&input.inverse_name)
    };
    conn.execute(
        "UPDATE relation_types SET name = ?1, inverse_name = ?2, directionality = ?3 WHERE id = ?4",
        params![name, inverse, input.directionality, current.id],
    )?;
    get_type(conn, &current.id)
}

/// Só tipos sem nenhuma relação podem ser excluídos.
pub fn delete_type(conn: &Connection, id: &str) -> Result<()> {
    let t = get_type(conn, id)?;
    if t.usage_count > 0 {
        return Err(AppError::msg(format!(
            "este tipo é usado por {} relação(ões); remova-as antes \
             (relações geradas por propriedades somem ao limpar ou apagar a propriedade)",
            t.usage_count
        )));
    }
    conn.execute("DELETE FROM relation_types WHERE id = ?1", [id])?;
    Ok(())
}

/// Devolve o id do tipo com esse nome, criando-o se não existir.
pub fn ensure_type(
    conn: &Connection,
    universe_id: &str,
    name: &str,
    inverse: Option<&str>,
    directionality: &str,
) -> Result<String> {
    let found: Option<String> = conn
        .query_row(
            "SELECT id FROM relation_types
             WHERE universe_id = ?1 AND name = ?2 COLLATE NOCASE",
            params![universe_id, name],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = found {
        return Ok(id);
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO relation_types (id, universe_id, name, inverse_name, directionality)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, universe_id, name, inverse, directionality],
    )?;
    Ok(id)
}

// ---------- Relações ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRelation {
    pub universe_id: String,
    pub type_id: String,
    pub source_id: String,
    pub target_id: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Relação vista a partir de uma entidade.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationView {
    pub id: String,
    pub type_id: String,
    /// Rótulo do ponto de vista da entidade consultada (nome ou nome inverso).
    pub label: String,
    /// "outgoing" | "incoming" | "both"
    pub direction: String,
    pub directionality: String,
    pub other: EntityRef,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// Preenchido quando a relação vem de uma propriedade entity_ref.
    pub property_key: Option<String>,
}

pub fn create(conn: &Connection, input: NewRelation) -> Result<String> {
    if input.source_id == input.target_id {
        return Err(AppError::msg("uma entidade não pode se relacionar consigo mesma"));
    }
    let in_universe: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE id IN (?1, ?2) AND universe_id = ?3",
        params![input.source_id, input.target_id, input.universe_id],
        |r| r.get(0),
    )?;
    if in_universe != 2 {
        return Err(AppError::msg("origem e destino precisam existir neste universo"));
    }
    let t = get_type(conn, &input.type_id)?;
    if t.universe_id != input.universe_id {
        return Err(AppError::msg("o tipo de relação pertence a outro universo"));
    }

    let description = clean(&input.description);
    let start = clean(&input.start_date);
    let end = clean(&input.end_date);

    // Evita duplicatas exatas (e a inversa, quando a relação é bidirecional).
    let dup: i64 = conn.query_row(
        "SELECT COUNT(*) FROM relations
         WHERE type_id = ?1 AND property_key IS NULL
           AND ((source_id = ?2 AND target_id = ?3)
                OR (?4 = 1 AND source_id = ?3 AND target_id = ?2))
           AND COALESCE(start_date, '') = ?5 AND COALESCE(end_date, '') = ?6",
        params![
            t.id,
            input.source_id,
            input.target_id,
            (t.directionality == "bidirectional") as i64,
            start.clone().unwrap_or_default(),
            end.clone().unwrap_or_default()
        ],
        |r| r.get(0),
    )?;
    if dup > 0 {
        return Err(AppError::msg("essa relação já existe"));
    }

    let id = new_id();
    conn.execute(
        "INSERT INTO relations
           (id, universe_id, type_id, source_id, target_id, description, start_date, end_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            input.universe_id,
            t.id,
            input.source_id,
            input.target_id,
            description,
            start,
            end
        ],
    )?;
    Ok(id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let property_key: Option<String> = conn
        .query_row(
            "SELECT property_key FROM relations WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::msg("relação não encontrada"))?;
    if property_key.is_some() {
        return Err(AppError::msg(
            "essa relação vem de uma propriedade: edite a propriedade da entidade",
        ));
    }
    conn.execute("DELETE FROM relations WHERE id = ?1", [id])?;
    Ok(())
}

pub fn list_for_entity(conn: &Connection, entity_id: &str) -> Result<Vec<RelationView>> {
    let mut st = conn.prepare(
        "SELECT r.id, r.type_id, t.name, t.inverse_name, t.directionality,
                r.source_id, r.description, r.start_date, r.end_date, r.property_key,
                o.id, o.name, o.type_id
         FROM relations r
         JOIN relation_types t ON t.id = r.type_id
         JOIN entities o ON o.id = CASE WHEN r.source_id = ?1 THEN r.target_id ELSE r.source_id END
         WHERE r.source_id = ?1 OR r.target_id = ?1
         ORDER BY t.name COLLATE NOCASE, o.name COLLATE NOCASE",
    )?;
    let rows = st.query_map([entity_id], |r| {
        let name: String = r.get(2)?;
        let inverse: Option<String> = r.get(3)?;
        let directionality: String = r.get(4)?;
        let source_id: String = r.get(5)?;
        let outgoing = source_id == entity_id;

        let (direction, label) = if directionality == "bidirectional" {
            ("both", name)
        } else if outgoing {
            ("outgoing", name)
        } else {
            let label = inverse.unwrap_or_else(|| format!("{name} (inverso)"));
            ("incoming", label)
        };

        Ok(RelationView {
            id: r.get(0)?,
            type_id: r.get(1)?,
            label,
            direction: direction.to_string(),
            directionality,
            description: r.get(6)?,
            start_date: r.get(7)?,
            end_date: r.get(8)?,
            property_key: r.get(9)?,
            other: EntityRef {
                id: r.get(10)?,
                name: r.get(11)?,
                type_id: r.get(12)?,
            },
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}