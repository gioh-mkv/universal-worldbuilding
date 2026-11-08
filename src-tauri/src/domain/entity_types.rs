use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::entities;
use crate::db::new_id;
use crate::error::{AppError, Result};

fn empty_obj() -> Value {
    json!({})
}

/// Chaves viram caminhos JSON: só letras ASCII, números e "_".
fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPropertyDef {
    pub key: String,
    pub label: String,
    pub data_type: String,
    #[serde(default = "empty_obj")]
    pub config: Value,
    #[serde(default)]
    pub required: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEntityType {
    pub universe_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    #[serde(default)]
    pub properties: Vec<NewPropertyDef>,
}

/// Propriedade na edição de um tipo. A ordem da lista define a posição.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyEdit {
    /// Id da propriedade existente; ausente = propriedade nova.
    pub id: Option<String>,
    /// Só para propriedades novas (chave e tipo de dado não mudam depois).
    pub key: Option<String>,
    pub data_type: Option<String>,
    pub label: String,
    #[serde(default = "empty_obj")]
    pub config: Value,
    #[serde(default)]
    pub required: bool,
}

/// Propriedades existentes que não estiverem em `properties` são REMOVIDAS
/// (junto com os valores delas nas entidades).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntityType {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub properties: Vec<PropertyEdit>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDef {
    pub id: String,
    pub key: String,
    pub label: String,
    pub data_type: String,
    pub config: Value,
    pub required: bool,
    pub position: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityType {
    pub id: String,
    pub universe_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub properties: Vec<PropertyDef>,
}

fn ensure_unique_name(
    conn: &Connection,
    universe_id: &str,
    name: &str,
    except_id: Option<&str>,
) -> Result<()> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entity_types
         WHERE universe_id = ?1 AND name = ?2 COLLATE NOCASE AND (?3 IS NULL OR id <> ?3)",
        params![universe_id, name, except_id],
        |r| r.get(0),
    )?;
    if n > 0 {
        return Err(AppError::msg("já existe um tipo com esse nome neste universo"));
    }
    Ok(())
}

pub fn create(conn: &Connection, input: NewEntityType) -> Result<EntityType> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::msg("o tipo precisa de um nome"));
    }
    ensure_unique_name(conn, &input.universe_id, name, None)?;
    let mut seen = HashSet::new();
    for p in &input.properties {
        let k = p.key.trim();
        if !valid_key(k) {
            return Err(AppError::msg(format!("chave de propriedade inválida: \"{k}\"")));
        }
        if !seen.insert(k.to_string()) {
            return Err(AppError::msg(format!("chave de propriedade repetida: \"{k}\"")));
        }
    }

    let tx = conn.unchecked_transaction()?;
    let id = new_id();
    let position: i64 = tx.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM entity_types WHERE universe_id = ?1",
        [&input.universe_id],
        |r| r.get(0),
    )?;
    tx.execute(
        "INSERT INTO entity_types (id, universe_id, name, icon, color, position)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, input.universe_id, name, input.icon, input.color, position],
    )?;
    for (i, p) in input.properties.iter().enumerate() {
        tx.execute(
            "INSERT INTO property_defs (id, type_id, key, label, data_type, config, required, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id(),
                id,
                p.key.trim(),
                p.label.trim(),
                p.data_type,
                p.config.to_string(),
                p.required,
                i as i64
            ],
        )?;
    }
    tx.commit()?;
    get(conn, &id)
}

pub fn property_defs(conn: &Connection, type_id: &str) -> Result<Vec<PropertyDef>> {
    let mut st = conn.prepare(
        "SELECT id, key, label, data_type, config, required, position
         FROM property_defs WHERE type_id = ?1 ORDER BY position",
    )?;
    let rows = st.query_map([type_id], |r| {
        let cfg: String = r.get(4)?;
        Ok(PropertyDef {
            id: r.get(0)?,
            key: r.get(1)?,
            label: r.get(2)?,
            data_type: r.get(3)?,
            config: serde_json::from_str(&cfg).unwrap_or(Value::Null),
            required: r.get::<_, i64>(5)? != 0,
            position: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, id: &str) -> Result<EntityType> {
    let row = conn
        .query_row(
            "SELECT id, universe_id, name, icon, color FROM entity_types WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::msg("tipo de entidade não encontrado"))?;
    Ok(EntityType {
        properties: property_defs(conn, &row.0)?,
        id: row.0,
        universe_id: row.1,
        name: row.2,
        icon: row.3,
        color: row.4,
    })
}

pub fn list(conn: &Connection, universe_id: &str) -> Result<Vec<EntityType>> {
    let mut st = conn
        .prepare("SELECT id FROM entity_types WHERE universe_id = ?1 ORDER BY position")?;
    let ids = st
        .query_map([universe_id], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter().map(|id| get(conn, id)).collect()
}

/// Remove a propriedade, os valores dela nas entidades e as relações que ela gerou.
fn drop_property(conn: &Connection, type_id: &str, d: &PropertyDef) -> Result<()> {
    let path = format!("$.\"{}\"", d.key);
    conn.execute(
        "DELETE FROM relations
         WHERE property_key = ?1 AND source_id IN (SELECT id FROM entities WHERE type_id = ?2)",
        params![d.key, type_id],
    )?;
    conn.execute(
        "UPDATE entities SET properties = json_remove(properties, ?1) WHERE type_id = ?2",
        params![path, type_id],
    )?;
    conn.execute("DELETE FROM property_defs WHERE id = ?1", [&d.id])?;
    entities::reindex_type(conn, type_id)?;
    Ok(())
}

/// Depois de mudar as opções de uma lista, limpa dos registros os valores
/// que deixaram de existir.
fn prune_options(conn: &Connection, type_id: &str, key: &str, config: &Value) -> Result<()> {
    let Some(options) = config.get("options").and_then(Value::as_array) else {
        return Ok(());
    };
    let allowed: Vec<&str> = options.iter().filter_map(Value::as_str).collect();

    let rows: Vec<(String, String)> = {
        let mut st = conn.prepare("SELECT id, properties FROM entities WHERE type_id = ?1")?;
        let it = st.query_map([type_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        it.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let mut changed = false;
    for (id, props) in rows {
        let mut value: Value = serde_json::from_str(&props)?;
        let Some(obj) = value.as_object_mut() else {
            continue;
        };
        let Some(current) = obj.get(key).cloned() else {
            continue;
        };
        let replacement: Option<Option<Value>> = match &current {
            Value::String(s) if !allowed.contains(&s.as_str()) => Some(None),
            Value::Array(a) => {
                let kept: Vec<Value> = a
                    .iter()
                    .filter(|x| x.as_str().map(|s| allowed.contains(&s)).unwrap_or(false))
                    .cloned()
                    .collect();
                if kept.len() == a.len() {
                    None
                } else if kept.is_empty() {
                    Some(None)
                } else {
                    Some(Some(Value::Array(kept)))
                }
            }
            _ => None,
        };
        if let Some(new_value) = replacement {
            match new_value {
                Some(v) => {
                    obj.insert(key.to_string(), v);
                }
                None => {
                    obj.remove(key);
                }
            }
            conn.execute(
                "UPDATE entities SET properties = ?1 WHERE id = ?2",
                params![value.to_string(), id],
            )?;
            changed = true;
        }
    }
    if changed {
        entities::reindex_type(conn, type_id)?;
    }
    Ok(())
}

pub fn update(conn: &Connection, input: UpdateEntityType) -> Result<EntityType> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::msg("o tipo precisa de um nome"));
    }
    let tx = conn.unchecked_transaction()?;
    let current = get(&tx, &input.id)?;
    ensure_unique_name(&tx, &current.universe_id, name, Some(&current.id))?;

    let existing = &current.properties;
    let kept: HashSet<&str> = input
        .properties
        .iter()
        .filter_map(|p| p.id.as_deref())
        .collect();
    for id in &kept {
        if !existing.iter().any(|d| d.id == *id) {
            return Err(AppError::msg("propriedade desconhecida neste tipo"));
        }
    }

    tx.execute(
        "UPDATE entity_types SET name = ?1, icon = ?2, color = ?3 WHERE id = ?4",
        params![name, input.icon, input.color, current.id],
    )?;

    // 1) Remove as propriedades que saíram da lista.
    for d in existing.iter().filter(|d| !kept.contains(d.id.as_str())) {
        drop_property(&tx, &current.id, d)?;
    }

    // 2) Atualiza as existentes e cria as novas, na ordem recebida.
    let mut used: HashSet<String> = existing
        .iter()
        .filter(|d| kept.contains(d.id.as_str()))
        .map(|d| d.key.clone())
        .collect();

    for (i, p) in input.properties.iter().enumerate() {
        let label = p.label.trim();
        if label.is_empty() {
            return Err(AppError::msg("toda propriedade precisa de um nome"));
        }
        match &p.id {
            Some(id) => {
                let def = existing
                    .iter()
                    .find(|d| &d.id == id)
                    .ok_or_else(|| AppError::msg("propriedade desconhecida neste tipo"))?;
                tx.execute(
                    "UPDATE property_defs
                     SET label = ?1, config = ?2, required = ?3, position = ?4
                     WHERE id = ?5",
                    params![label, p.config.to_string(), p.required, i as i64, id],
                )?;
                if matches!(def.data_type.as_str(), "select" | "multiselect") {
                    prune_options(&tx, &current.id, &def.key, &p.config)?;
                }
            }
            None => {
                let key = p
                    .key
                    .as_deref()
                    .map(str::trim)
                    .filter(|k| valid_key(k))
                    .ok_or_else(|| AppError::msg("chave de propriedade inválida"))?;
                let data_type = p
                    .data_type
                    .as_deref()
                    .ok_or_else(|| AppError::msg("propriedade nova sem tipo de dado"))?;
                if !used.insert(key.to_string()) {
                    return Err(AppError::msg(format!("chave de propriedade repetida: \"{key}\"")));
                }
                tx.execute(
                    "INSERT INTO property_defs
                       (id, type_id, key, label, data_type, config, required, position)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        new_id(),
                        current.id,
                        key,
                        label,
                        data_type,
                        p.config.to_string(),
                        p.required,
                        i as i64
                    ],
                )?;
            }
        }
    }

    tx.commit()?;
    get(conn, &input.id)
}

/// Só tipos sem entidades podem ser excluídos.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE type_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if n > 0 {
        return Err(AppError::msg(format!(
            "este tipo possui {n} entidade(s); exclua-as antes de excluir o tipo"
        )));
    }
    let tx = conn.unchecked_transaction()?;
    // Outras propriedades que restringiam referências a este tipo ficam sem restrição.
    tx.execute(
        "UPDATE property_defs SET config = json_remove(config, '$.targetTypeId')
         WHERE json_extract(config, '$.targetTypeId') = ?1",
        [id],
    )?;
    let changed = tx.execute("DELETE FROM entity_types WHERE id = ?1", [id])?; // cascata: property_defs
    if changed == 0 {
        return Err(AppError::msg("tipo de entidade não encontrado"));
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::universes;
    use std::path::Path;

    #[test]
    fn editar_tipo_remove_propriedade_poda_opcoes_e_protege_exclusao() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let u = universes::create(&conn, "U", None).unwrap();
        let t = create(
            &conn,
            serde_json::from_value(json!({
                "universeId": u.id, "name": "Personagem",
                "properties": [
                    {"key": "idade", "label": "Idade", "dataType": "number"},
                    {"key": "especie", "label": "Espécie", "dataType": "select",
                     "config": {"options": ["Humano", "Elfo"]}}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let ent = entities::create(
            &conn,
            serde_json::from_value(json!({
                "universeId": u.id, "typeId": t.id, "name": "Elena",
                "properties": {"idade": 41, "especie": "Elfo"}
            }))
            .unwrap(),
        )
        .unwrap();

        // Remove "Idade" e deixa só "Humano" como opção de espécie.
        let especie_id = t.properties[1].id.clone();
        let updated = update(
            &conn,
            serde_json::from_value(json!({
                "id": t.id, "name": "Personagem",
                "properties": [
                    {"id": especie_id, "label": "Espécie", "config": {"options": ["Humano"]}}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(updated.properties.len(), 1);

        let ent = entities::get(&conn, &ent.id).unwrap();
        assert!(ent.properties.get("idade").is_none());
        assert!(ent.properties.get("especie").is_none());

        // Com entidades, o tipo não pode ser excluído.
        assert!(delete(&conn, &t.id).is_err());
        entities::delete(&conn, &ent.id).unwrap();
        delete(&conn, &t.id).unwrap();
    }
}