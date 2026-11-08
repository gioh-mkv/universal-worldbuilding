use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::entity_types::{property_defs, PropertyDef};
use super::{relations, richtext};
use crate::db::new_id;
use crate::error::{AppError, Result};

fn empty_obj() -> Value {
    json!({})
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: String,
    pub universe_id: String,
    pub type_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub properties: Value,
    /// Documento TipTap (fonte da verdade).
    pub body_json: Option<Value>,
    /// Espelho em Markdown, gerado no backend (pesquisa e exportação).
    pub body_md: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEntity {
    pub universe_id: String,
    pub type_id: String,
    pub name: String,
    pub summary: Option<String>,
    #[serde(default = "empty_obj")]
    pub properties: Value,
    pub body_json: Option<Value>,
}

/// Campos ausentes (null) permanecem como estão. Para limpar um texto, envie "".
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntity {
    pub id: String,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub properties: Option<Value>,
    pub body_json: Option<Value>,
}

const COLS: &str = "id, universe_id, type_id, name, summary, properties, body_json, body_md, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Entity> {
    let props: String = r.get(5)?;
    let body_json: Option<String> = r.get(6)?;
    Ok(Entity {
        id: r.get(0)?,
        universe_id: r.get(1)?,
        type_id: r.get(2)?,
        name: r.get(3)?,
        summary: r.get(4)?,
        properties: serde_json::from_str(&props).unwrap_or(Value::Null),
        body_json: body_json.and_then(|s| serde_json::from_str(&s).ok()),
        body_md: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<Entity> {
    conn.query_row(
        &format!("SELECT {COLS} FROM entities WHERE id = ?1"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::msg("entidade não encontrada"))
}

pub fn list(conn: &Connection, universe_id: &str, type_id: Option<&str>) -> Result<Vec<Entity>> {
    let mut st = conn.prepare(&format!(
        "SELECT {COLS} FROM entities
         WHERE universe_id = ?1 AND (?2 IS NULL OR type_id = ?2)
         ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = st.query_map(params![universe_id, type_id], from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// A referência precisa existir no mesmo universo e, se o campo definir
/// `targetTypeId`, ser de um tipo específico.
fn check_ref(conn: &Connection, type_id: &str, target: &str, d: &PropertyDef) -> Result<()> {
    let found: Option<String> = conn
        .query_row(
            "SELECT type_id FROM entities
             WHERE id = ?1
               AND universe_id = (SELECT universe_id FROM entity_types WHERE id = ?2)",
            params![target, type_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(found) = found else {
        return Err(AppError::msg(format!(
            "\"{}\" aponta para uma entidade inexistente",
            d.label
        )));
    };
    if let Some(expected) = d.config.get("targetTypeId").and_then(Value::as_str) {
        if expected != found {
            return Err(AppError::msg(format!(
                "\"{}\" não aceita entidades desse tipo",
                d.label
            )));
        }
    }
    Ok(())
}

/// Sem lista de opções definida, qualquer texto é aceito.
fn in_options(d: &PropertyDef, s: &str) -> bool {
    match d.config.get("options").and_then(Value::as_array) {
        Some(opts) => opts.iter().any(|o| o.as_str() == Some(s)),
        None => true,
    }
}

/// Valida as propriedades contra as definições do tipo.
fn validate(conn: &Connection, type_id: &str, props: &Value) -> Result<()> {
    let obj = props
        .as_object()
        .ok_or_else(|| AppError::msg("properties deve ser um objeto"))?;
    let defs = property_defs(conn, type_id)?;

    for key in obj.keys() {
        if !defs.iter().any(|d| &d.key == key) {
            return Err(AppError::msg(format!("propriedade desconhecida: {key}")));
        }
    }
    for d in &defs {
        match obj.get(&d.key) {
            None | Some(Value::Null) => {
                if d.required {
                    return Err(AppError::msg(format!("propriedade obrigatória: {}", d.label)));
                }
            }
            Some(v) => {
                let ok = match d.data_type.as_str() {
                    "number" => v.is_number(),
                    "boolean" => v.is_boolean(),
                    "select" => v.as_str().map(|s| in_options(d, s)).unwrap_or(false),
                    "multiselect" => v
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .all(|x| x.as_str().map(|s| in_options(d, s)).unwrap_or(false))
                        })
                        .unwrap_or(false),
                    "coordinates" => v.is_object() || v.is_array(),
                    "computed" => true,
                    // text, date, datetime, image, file, entity_ref
                    _ => v.is_string(),
                };
                if !ok {
                    return Err(AppError::msg(format!(
                        "valor inválido para \"{}\" (esperado: {})",
                        d.label, d.data_type
                    )));
                }
                if d.data_type == "entity_ref" {
                    if let Some(target) = v.as_str() {
                        check_ref(conn, type_id, target, d)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_doc(doc: &Value) -> Result<()> {
    if doc.get("type").and_then(Value::as_str) == Some("doc") {
        Ok(())
    } else {
        Err(AppError::msg("documento inválido: esperado um nó \"doc\""))
    }
}

fn flatten(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(flatten).collect::<Vec<_>>().join(" "),
        other => other.to_string(),
    }
}

/// Mantém o índice FTS5 sincronizado com a entidade.
pub fn reindex(conn: &Connection, e: &Entity) -> Result<()> {
    let props = match &e.properties {
        Value::Object(m) => m.values().map(flatten).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    };
    conn.execute("DELETE FROM search_index WHERE entity_id = ?1", [&e.id])?;
    conn.execute(
        "INSERT INTO search_index (entity_id, universe_id, name, summary, body, props)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            e.id,
            e.universe_id,
            e.name,
            e.summary.clone().unwrap_or_default(),
            e.body_md.clone().unwrap_or_default(),
            props
        ],
    )?;
    Ok(())
}

/// Reindexa todas as entidades de um tipo (usado ao editar as propriedades do tipo).
pub fn reindex_type(conn: &Connection, type_id: &str) -> Result<()> {
    let ids = {
        let mut st = conn.prepare("SELECT id FROM entities WHERE type_id = ?1")?;
        let rows = st.query_map([type_id], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for id in ids {
        let e = get(conn, &id)?;
        reindex(conn, &e)?;
    }
    Ok(())
}

/// Recria as linhas de `mentions` a partir do documento.
/// O SELECT garante que o alvo exista e pertença ao mesmo universo.
pub fn sync_mentions(conn: &Connection, e: &Entity) -> Result<()> {
    conn.execute("DELETE FROM mentions WHERE source_id = ?1", [&e.id])?;
    if let Some(doc) = &e.body_json {
        for target in richtext::mention_ids(doc) {
            if target == e.id {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO mentions (source_id, target_id)
                 SELECT ?1, id FROM entities WHERE id = ?2 AND universe_id = ?3",
                params![e.id, target, e.universe_id],
            )?;
        }
    }
    Ok(())
}

/// Propriedades entity_ref viram relações reais (property_key preenchido).
/// As relações manuais (property_key NULL) não são tocadas.
fn sync_property_relations(conn: &Connection, e: &Entity) -> Result<()> {
    conn.execute(
        "DELETE FROM relations WHERE source_id = ?1 AND property_key IS NOT NULL",
        [&e.id],
    )?;
    let defs = property_defs(conn, &e.type_id)?;
    for d in defs.iter().filter(|d| d.data_type == "entity_ref") {
        let Some(target) = e.properties.get(&d.key).and_then(Value::as_str) else {
            continue;
        };
        if target == e.id {
            continue;
        }
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entities WHERE id = ?1 AND universe_id = ?2",
            params![target, e.universe_id],
            |r| r.get(0),
        )?;
        if exists == 0 {
            continue;
        }
        let inverse = format!("{} de", d.label);
        let type_id = relations::ensure_type(
            conn,
            &e.universe_id,
            &d.label,
            Some(inverse.as_str()),
            "directed",
        )?;
        conn.execute(
            "INSERT INTO relations (id, universe_id, type_id, source_id, target_id, property_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![new_id(), e.universe_id, type_id, e.id, target, d.key],
        )?;
    }
    Ok(())
}

/// Ids das entidades cujos documentos mencionam `target_id`.
fn mention_sources(conn: &Connection, target_id: &str) -> Result<Vec<String>> {
    let mut st = conn.prepare("SELECT source_id FROM mentions WHERE target_id = ?1")?;
    let rows = st.query_map([target_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Depois de renomear uma entidade, corrige o rótulo das menções a ela nos
/// documentos de outras entidades (JSON, Markdown e índice de pesquisa).
/// É uma alteração derivada: não cria versão nem muda `updated_at`.
fn propagate_rename(conn: &Connection, renamed: &Entity) -> Result<()> {
    for sid in mention_sources(conn, &renamed.id)? {
        let src = get(conn, &sid)?;
        let Some(mut doc) = src.body_json.clone() else {
            continue;
        };
        if richtext::rename_mention(&mut doc, &renamed.id, &renamed.name) {
            let md = richtext::to_markdown(&doc);
            conn.execute(
                "UPDATE entities SET body_json = ?1, body_md = ?2 WHERE id = ?3",
                params![doc.to_string(), md, sid],
            )?;
            let updated = get(conn, &sid)?;
            reindex(conn, &updated)?;
        }
    }
    Ok(())
}

pub fn create(conn: &Connection, input: NewEntity) -> Result<Entity> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::msg("a entidade precisa de um nome"));
    }
    let belongs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entity_types WHERE id = ?1 AND universe_id = ?2",
        params![input.type_id, input.universe_id],
        |r| r.get(0),
    )?;
    if belongs == 0 {
        return Err(AppError::msg("o tipo não pertence a este universo"));
    }
    validate(conn, &input.type_id, &input.properties)?;
    if let Some(d) = &input.body_json {
        validate_doc(d)?;
    }

    let body_md = input.body_json.as_ref().map(richtext::to_markdown);
    let body_json = input.body_json.as_ref().map(|d| d.to_string());

    let id = new_id();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO entities (id, universe_id, type_id, name, summary, properties, body_json, body_md)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            input.universe_id,
            input.type_id,
            name,
            input.summary,
            input.properties.to_string(),
            body_json,
            body_md
        ],
    )?;
    let e = get(&tx, &id)?;
    reindex(&tx, &e)?;
    sync_mentions(&tx, &e)?;
    sync_property_relations(&tx, &e)?;
    tx.commit()?;
    Ok(e)
}

pub fn update(conn: &Connection, input: UpdateEntity) -> Result<Entity> {
    let tx = conn.unchecked_transaction()?;
    let cur = get(&tx, &input.id)?;

    // Snapshot do estado anterior (base do histórico/restauração).
    tx.execute(
        "INSERT INTO entity_versions (id, entity_id, snapshot) VALUES (?1, ?2, ?3)",
        params![new_id(), cur.id, serde_json::to_string(&cur)?],
    )?;

    let name = input.name.unwrap_or_else(|| cur.name.clone());
    if name.trim().is_empty() {
        return Err(AppError::msg("a entidade precisa de um nome"));
    }
    let summary = input.summary.or_else(|| cur.summary.clone());
    let properties = input.properties.unwrap_or_else(|| cur.properties.clone());
    validate(&tx, &cur.type_id, &properties)?;

    // Documento novo recalcula o Markdown; sem documento novo, mantém o atual.
    let (body_json, body_md) = match &input.body_json {
        Some(d) => {
            validate_doc(d)?;
            (Some(d.clone()), Some(richtext::to_markdown(d)))
        }
        None => (cur.body_json.clone(), cur.body_md.clone()),
    };

    tx.execute(
        "UPDATE entities
         SET name = ?1, summary = ?2, properties = ?3, body_json = ?4, body_md = ?5,
             updated_at = datetime('now')
         WHERE id = ?6",
        params![
            name.trim(),
            summary,
            properties.to_string(),
            body_json.as_ref().map(|d| d.to_string()),
            body_md,
            cur.id
        ],
    )?;
    let e = get(&tx, &cur.id)?;
    reindex(&tx, &e)?;
    sync_mentions(&tx, &e)?;
    sync_property_relations(&tx, &e)?;
    if e.name != cur.name {
        propagate_rename(&tx, &e)?;
    }
    tx.commit()?;
    Ok(e)
}

/// Exclui a entidade e mantém o restante consistente:
/// 1. menções a ela viram texto simples nos outros documentos;
/// 2. propriedades entity_ref que apontavam para ela são limpas;
/// 3. relações, menções e versões dela somem (ON DELETE CASCADE) e sai do índice.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let e = get(&tx, id)?;

    for sid in mention_sources(&tx, &e.id)? {
        let src = get(&tx, &sid)?;
        let Some(mut doc) = src.body_json.clone() else {
            continue;
        };
        if richtext::unlink_mention(&mut doc, &e.id) {
            let md = richtext::to_markdown(&doc);
            tx.execute(
                "UPDATE entities SET body_json = ?1, body_md = ?2 WHERE id = ?3",
                params![doc.to_string(), md, sid],
            )?;
            let updated = get(&tx, &sid)?;
            reindex(&tx, &updated)?;
        }
    }

    let ref_defs: Vec<(String, String)> = {
        let mut st = tx.prepare(
            "SELECT d.type_id, d.key
             FROM property_defs d JOIN entity_types t ON t.id = d.type_id
             WHERE d.data_type = 'entity_ref' AND t.universe_id = ?1",
        )?;
        let rows = st.query_map([&e.universe_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (type_id, key) in ref_defs {
        let path = format!("$.\"{key}\"");
        let affected: Vec<String> = {
            let mut st = tx.prepare(
                "SELECT id FROM entities WHERE type_id = ?1 AND json_extract(properties, ?2) = ?3",
            )?;
            let rows = st.query_map(params![type_id, path, e.id], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for aid in affected {
            tx.execute(
                "UPDATE entities SET properties = json_remove(properties, ?1) WHERE id = ?2",
                params![path, aid],
            )?;
            let updated = get(&tx, &aid)?;
            reindex(&tx, &updated)?;
        }
    }

    tx.execute("DELETE FROM search_index WHERE entity_id = ?1", [&e.id])?;
    tx.execute("DELETE FROM entities WHERE id = ?1", [&e.id])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{entity_types, universes};
    use std::path::Path;

    #[test]
    fn renomear_atualiza_documentos_que_mencionam() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let u = universes::create(&conn, "U", None).unwrap();
        let t = entity_types::create(
            &conn,
            serde_json::from_value(json!({"universeId": u.id, "name": "Tipo"})).unwrap(),
        )
        .unwrap();
        let new = |name: &str, doc: Option<Value>| {
            serde_json::from_value::<NewEntity>(json!({
                "universeId": u.id, "typeId": t.id, "name": name, "bodyJson": doc
            }))
            .unwrap()
        };

        let elena = create(&conn, new("Elena", None)).unwrap();
        let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[
            {"type":"mention","attrs":{"id": elena.id, "label":"Elena"}}]}]});
        let aurora = create(&conn, new("Aurora", Some(doc))).unwrap();

        update(
            &conn,
            UpdateEntity {
                id: elena.id.clone(),
                name: Some("Elena V.".into()),
                summary: None,
                properties: None,
                body_json: None,
            },
        )
        .unwrap();

        let aurora = get(&conn, &aurora.id).unwrap();
        assert_eq!(aurora.body_md.as_deref(), Some("[[Elena V.]]"));
    }

    fn mk(
        conn: &Connection,
        universe_id: &str,
        type_id: &str,
        name: &str,
        doc: Option<Value>,
        props: Value,
    ) -> Entity {
        create(
            conn,
            serde_json::from_value::<NewEntity>(json!({
                "universeId": universe_id, "typeId": type_id, "name": name,
                "bodyJson": doc, "properties": props
            }))
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn excluir_desfaz_mencoes_e_referencias() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let u = universes::create(&conn, "U", None).unwrap();
        let t = entity_types::create(
            &conn,
            serde_json::from_value(json!({
                "universeId": u.id, "name": "Tipo",
                "properties": [{"key": "amigo", "label": "Amigo", "dataType": "entity_ref"}]
            }))
            .unwrap(),
        )
        .unwrap();

        let elena = mk(&conn, &u.id, &t.id, "Elena", None, json!({}));
        let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[
            {"type":"text","text":"Aliada de "},
            {"type":"mention","attrs":{"id": elena.id, "label":"Elena"}}]}]});
        let aurora = mk(&conn, &u.id, &t.id, "Aurora", Some(doc), json!({"amigo": elena.id}));

        delete(&conn, &elena.id).unwrap();

        assert!(get(&conn, &elena.id).is_err());
        let aurora = get(&conn, &aurora.id).unwrap();
        assert_eq!(aurora.body_md.as_deref(), Some("Aliada de Elena"));
        assert!(aurora.properties.get("amigo").is_none());

        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(count("mentions"), 0);
        assert_eq!(count("relations"), 0);
        assert_eq!(count("search_index"), 1);
    }
}