//! Exportação (JSON e Markdown) e importação (JSON) de universos.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::entities::{self, Entity};
use super::entity_types::{self, EntityType};
use super::{relations, richtext, universes};
use crate::db::new_id;
use crate::error::{AppError, Result};

pub const FORMAT: &str = "uwb-universe";
pub const FORMAT_VERSION: u32 = 1;
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_WARNINGS: usize = 50;
const DATA_TYPES: [&str; 12] = [
    "text", "number", "date", "datetime", "boolean", "select", "multiselect", "image", "file",
    "entity_ref", "coordinates", "computed",
];
const DIRECTIONS: [&str; 3] = ["directed", "bidirectional", "hierarchical"];

fn empty_obj() -> Value {
    json!({})
}

fn directed() -> String {
    "directed".to_string()
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
}

/// Mesma regra de entity_types::valid_key (chaves viram caminhos JSON). Mantenha em sincronia.
fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---------- formato do arquivo ----------

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseExport {
    pub format: String,
    pub format_version: u32,
    #[serde(default)]
    pub exported_at: String,
    #[serde(default)]
    pub app_version: String,
    pub universe: ExportUniverse,
    #[serde(default)]
    pub entity_types: Vec<ExportEntityType>,
    #[serde(default)]
    pub relation_types: Vec<ExportRelationType>,
    #[serde(default)]
    pub entities: Vec<ExportEntity>,
    #[serde(default)]
    pub relations: Vec<ExportRelation>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportUniverse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProperty {
    pub key: String,
    pub label: String,
    pub data_type: String,
    #[serde(default = "empty_obj")]
    pub config: Value,
    #[serde(default)]
    pub required: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEntityType {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    /// A ordem da lista é a ordem de exibição.
    #[serde(default)]
    pub properties: Vec<ExportProperty>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRelationType {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub inverse_name: Option<String>,
    #[serde(default = "directed")]
    pub directionality: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEntity {
    pub id: String,
    pub type_id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default = "empty_obj")]
    pub properties: Value,
    /// Documento do editor. As menções são recalculadas ao importar.
    #[serde(default)]
    pub body_json: Option<Value>,
    /// Só exportado para entidades antigas que ainda não têm `body_json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_md: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRelation {
    pub type_id: String,
    pub source_id: String,
    pub target_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    /// Preenchido quando a relação nasceu de uma propriedade entity_ref.
    #[serde(default)]
    pub property_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub entities: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub universe_id: String,
    pub universe_name: String,
    pub entity_types: usize,
    pub relation_types: usize,
    pub entities: usize,
    pub relations: usize,
    pub warnings: Vec<String>,
}

// ---------- exportação JSON ----------

fn find_universe(conn: &Connection, id: &str) -> Result<universes::Universe> {
    universes::list(conn)?
        .into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| AppError::msg("universo não encontrado"))
}

pub fn build_export(conn: &Connection, universe_id: &str) -> Result<UniverseExport> {
    let universe = find_universe(conn, universe_id)?;
    let exported_at: String = conn.query_row(
        "SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')",
        [],
        |r| r.get(0),
    )?;

    let types: Vec<ExportEntityType> = entity_types::list(conn, universe_id)?
        .into_iter()
        .map(|t| ExportEntityType {
            id: t.id,
            name: t.name,
            icon: t.icon,
            color: t.color,
            properties: t
                .properties
                .into_iter()
                .map(|p| ExportProperty {
                    key: p.key,
                    label: p.label,
                    data_type: p.data_type,
                    config: p.config,
                    required: p.required,
                })
                .collect(),
        })
        .collect();

    let rel_types: Vec<ExportRelationType> = relations::list_types(conn, universe_id)?
        .into_iter()
        .map(|t| ExportRelationType {
            id: t.id,
            name: t.name,
            inverse_name: t.inverse_name,
            directionality: t.directionality,
        })
        .collect();

    let ents: Vec<ExportEntity> = entities::list(conn, universe_id, None)?
        .into_iter()
        .map(|e| ExportEntity {
            id: e.id,
            type_id: e.type_id,
            name: e.name,
            summary: e.summary,
            properties: e.properties,
            body_md: if e.body_json.is_none() { e.body_md } else { None },
            body_json: e.body_json,
            created_at: Some(e.created_at),
            updated_at: Some(e.updated_at),
        })
        .collect();

    let rels: Vec<ExportRelation> = {
        let mut st = conn.prepare(
            "SELECT type_id, source_id, target_id, description, start_date, end_date, property_key
             FROM relations WHERE universe_id = ?1 ORDER BY id",
        )?;
        let rows = st.query_map([universe_id], |r| {
            Ok(ExportRelation {
                type_id: r.get(0)?,
                source_id: r.get(1)?,
                target_id: r.get(2)?,
                description: r.get(3)?,
                start_date: r.get(4)?,
                end_date: r.get(5)?,
                property_key: r.get(6)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    Ok(UniverseExport {
        format: FORMAT.to_string(),
        format_version: FORMAT_VERSION,
        exported_at,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        universe: ExportUniverse {
            id: universe.id,
            name: universe.name,
            description: universe.description,
        },
        entity_types: types,
        relation_types: rel_types,
        entities: ents,
        relations: rels,
    })
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn export_json(conn: &Connection, universe_id: &str, path: &Path) -> Result<ExportResult> {
    let data = build_export(conn, universe_id)?;
    let count = data.entities.len();
    write_atomic(path, serde_json::to_string_pretty(&data)?.as_bytes())?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        entities: count,
    })
}

// ---------- importação JSON ----------

/// Lê e valida o cabeçalho do arquivo (sem tocar no banco).
pub fn parse_export(path: &Path) -> Result<UniverseExport> {
    let meta = fs::metadata(path).map_err(|_| AppError::msg("arquivo não encontrado"))?;
    if !meta.is_file() {
        return Err(AppError::msg("o caminho escolhido não é um arquivo"));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(AppError::msg("arquivo grande demais (limite de 256 MB)"));
    }
    let raw = fs::read_to_string(path)
        .map_err(|_| AppError::msg("não foi possível ler o arquivo como texto UTF-8"))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|e| AppError::msg(format!("o arquivo não é um JSON válido: {e}")))?;
    if value.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(AppError::msg(
            "este arquivo não é uma exportação de universo do UWB",
        ));
    }
    let data: UniverseExport = serde_json::from_value(value)
        .map_err(|e| AppError::msg(format!("estrutura do arquivo inválida: {e}")))?;
    if data.format_version > FORMAT_VERSION {
        return Err(AppError::msg(
            "este arquivo foi criado por uma versão mais nova do UWB; atualize o aplicativo",
        ));
    }
    Ok(data)
}

#[derive(Default)]
struct Warnings {
    list: Vec<String>,
    extra: usize,
}

impl Warnings {
    fn add(&mut self, msg: String) {
        if self.list.len() < MAX_WARNINGS {
            self.list.push(msg);
        } else {
            self.extra += 1;
        }
    }

    fn finish(mut self) -> Vec<String> {
        if self.extra > 0 {
            self.list.push(format!("… e mais {} aviso(s)", self.extra));
        }
        self.list
    }
}

fn unique_names<'a>(names: impl Iterator<Item = &'a str>, what: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for n in names {
        let n = n.trim();
        if n.is_empty() {
            return Err(AppError::msg(format!("há {what} sem nome")));
        }
        if !seen.insert(n.to_lowercase()) {
            return Err(AppError::msg(format!("nome de {what} repetido: \"{n}\"")));
        }
    }
    Ok(())
}

fn pick_universe_name(conn: &Connection, requested: Option<&str>, original: &str) -> Result<String> {
    let base = requested
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| original.trim());
    let base = if base.is_empty() { "Universo importado" } else { base };

    let exists = |n: &str| -> Result<bool> {
        let c: i64 = conn.query_row(
            "SELECT COUNT(*) FROM universes WHERE name = ?1 COLLATE NOCASE",
            [n],
            |r| r.get(0),
        )?;
        Ok(c > 0)
    };
    if !exists(base)? {
        return Ok(base.to_string());
    }
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            format!("{base} (importado)")
        } else {
            format!("{base} (importado {n})")
        };
        if !exists(&candidate)? {
            return Ok(candidate);
        }
        n += 1;
    }
}

/// Reaponta as menções para os novos ids. Menções a entidades que não vieram no
/// arquivo viram texto simples. Devolve quantas ficaram pendentes.
fn remap_mentions(node: &mut Value, map: &HashMap<String, String>) -> usize {
    let mut dangling = 0;
    if let Some(children) = node.get_mut("content").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            if child.get("type").and_then(Value::as_str) == Some("mention") {
                let old = child
                    .pointer("/attrs/id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if let Some(new_id) = map.get(&old) {
                    child["attrs"]["id"] = Value::String(new_id.clone());
                } else {
                    let label = child
                        .pointer("/attrs/label")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    *child = json!({ "type": "text", "text": label });
                    dangling += 1;
                }
            } else {
                dangling += remap_mentions(child, map);
            }
        }
    }
    dangling
}

/// Cria um NOVO universo com o conteúdo do arquivo (ids regerados), tudo em uma
/// transação: se algo falhar, nada é criado.
pub fn import_universe(
    conn: &Connection,
    data: UniverseExport,
    new_name: Option<&str>,
) -> Result<ImportSummary> {
    if data.format != FORMAT {
        return Err(AppError::msg("formato de arquivo desconhecido"));
    }
    if data.format_version > FORMAT_VERSION {
        return Err(AppError::msg("arquivo criado por uma versão mais nova do UWB"));
    }

    let mut warnings = Warnings::default();

    // ---- validação estrutural (nada é gravado até aqui) ----
    let mut type_map: HashMap<String, String> = HashMap::new();
    for t in &data.entity_types {
        if type_map.insert(t.id.clone(), new_id()).is_some() {
            return Err(AppError::msg(format!("tipo de entidade com id repetido: {}", t.id)));
        }
    }
    unique_names(data.entity_types.iter().map(|t| t.name.as_str()), "tipo de entidade")?;
    for t in &data.entity_types {
        let mut keys: HashSet<&str> = HashSet::new();
        for p in &t.properties {
            if !valid_key(&p.key) {
                return Err(AppError::msg(format!(
                    "chave de propriedade inválida em \"{}\": \"{}\"",
                    t.name, p.key
                )));
            }
            if !keys.insert(p.key.as_str()) {
                return Err(AppError::msg(format!(
                    "chave de propriedade repetida em \"{}\": \"{}\"",
                    t.name, p.key
                )));
            }
            if p.label.trim().is_empty() {
                return Err(AppError::msg(format!("propriedade sem nome em \"{}\"", t.name)));
            }
            if !DATA_TYPES.contains(&p.data_type.as_str()) {
                return Err(AppError::msg(format!(
                    "tipo de dado desconhecido: \"{}\"",
                    p.data_type
                )));
            }
        }
    }

    let mut rel_type_map: HashMap<String, String> = HashMap::new();
    for t in &data.relation_types {
        if rel_type_map.insert(t.id.clone(), new_id()).is_some() {
            return Err(AppError::msg(format!("tipo de relação com id repetido: {}", t.id)));
        }
        if !DIRECTIONS.contains(&t.directionality.as_str()) {
            return Err(AppError::msg(format!(
                "direcionalidade inválida em \"{}\"",
                t.name
            )));
        }
    }
    unique_names(data.relation_types.iter().map(|t| t.name.as_str()), "tipo de relação")?;

    let mut entity_map: HashMap<String, String> = HashMap::new();
    for e in &data.entities {
        if entity_map.insert(e.id.clone(), new_id()).is_some() {
            return Err(AppError::msg(format!("entidade com id repetido: {}", e.id)));
        }
        if e.name.trim().is_empty() {
            return Err(AppError::msg("há entidade sem nome"));
        }
        if !type_map.contains_key(&e.type_id) {
            return Err(AppError::msg(format!(
                "a entidade \"{}\" aponta para um tipo que não está no arquivo",
                e.name
            )));
        }
    }

    let types_by_old: HashMap<&str, &ExportEntityType> = data
        .entity_types
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();
    let entity_type_of: HashMap<&str, &str> = data
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e.type_id.as_str()))
        .collect();

    // ---- gravação ----
    let tx = conn.unchecked_transaction()?;
    let universe_id = new_id();
    let universe_name = pick_universe_name(&tx, new_name, &data.universe.name)?;
    tx.execute(
        "INSERT INTO universes (id, name, description) VALUES (?1, ?2, ?3)",
        params![universe_id, universe_name, clean(&data.universe.description)],
    )?;

    for (i, t) in data.entity_types.iter().enumerate() {
        tx.execute(
            "INSERT INTO entity_types (id, universe_id, name, icon, color, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                type_map[&t.id],
                universe_id,
                t.name.trim(),
                clean(&t.icon),
                clean(&t.color),
                i as i64
            ],
        )?;
        for (j, p) in t.properties.iter().enumerate() {
            let mut config = if p.config.is_object() {
                p.config.clone()
            } else {
                json!({})
            };
            let target = config
                .get("targetTypeId")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(old) = target {
                match type_map.get(&old) {
                    Some(new_t) => config["targetTypeId"] = Value::String(new_t.clone()),
                    None => {
                        if let Some(o) = config.as_object_mut() {
                            o.remove("targetTypeId");
                        }
                        warnings.add(format!(
                            "\"{}\": a restrição de tipo de \"{}\" foi removida (tipo ausente)",
                            t.name, p.label
                        ));
                    }
                }
            }
            tx.execute(
                "INSERT INTO property_defs (id, type_id, key, label, data_type, config, required, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    new_id(),
                    type_map[&t.id],
                    p.key,
                    p.label.trim(),
                    p.data_type,
                    config.to_string(),
                    p.required,
                    j as i64
                ],
            )?;
        }
    }

    for t in &data.relation_types {
        let inverse = if t.directionality == "bidirectional" {
            None
        } else {
            clean(&t.inverse_name)
        };
        tx.execute(
            "INSERT INTO relation_types (id, universe_id, name, inverse_name, directionality)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rel_type_map[&t.id],
                universe_id,
                t.name.trim(),
                inverse,
                t.directionality
            ],
        )?;
    }

    let mut new_entity_ids: Vec<String> = Vec::with_capacity(data.entities.len());
    for e in &data.entities {
        let new_eid = entity_map[&e.id].clone();
        let new_tid = &type_map[&e.type_id];
        let t = types_by_old[e.type_id.as_str()];
        let label = e.name.trim();

        // Propriedades: descarta desconhecidas e remapeia referências.
        let mut props = serde_json::Map::new();
        match e.properties.as_object() {
            Some(obj) => {
                for (k, v) in obj {
                    let Some(def) = t.properties.iter().find(|p| &p.key == k) else {
                        warnings.add(format!(
                            "\"{label}\": propriedade desconhecida \"{k}\" ignorada"
                        ));
                        continue;
                    };
                    if v.is_null() {
                        continue;
                    }
                    if def.data_type == "entity_ref" {
                        match v.as_str().and_then(|old| entity_map.get(old)) {
                            Some(n) => {
                                props.insert(k.clone(), Value::String(n.clone()));
                            }
                            None => warnings.add(format!(
                                "\"{label}\": referência pendente em \"{}\" removida",
                                def.label
                            )),
                        }
                    } else {
                        props.insert(k.clone(), v.clone());
                    }
                }
            }
            None => warnings.add(format!("\"{label}\": propriedades inválidas ignoradas")),
        }

        // Documento: remapeia menções.
        let is_doc = e
            .body_json
            .as_ref()
            .map(|d| d.get("type").and_then(Value::as_str) == Some("doc"));
        let mut doc = match (e.body_json.clone(), is_doc) {
            (Some(d), Some(true)) => Some(d),
            (Some(_), _) => {
                warnings.add(format!("\"{label}\": documento inválido ignorado"));
                None
            }
            _ => None,
        };
        if let Some(d) = doc.as_mut() {
            let dangling = remap_mentions(d, &entity_map);
            if dangling > 0 {
                warnings.add(format!(
                    "\"{label}\": {dangling} menção(ões) a entidades ausentes viraram texto"
                ));
            }
        }
        let body_md = doc
            .as_ref()
            .map(richtext::to_markdown)
            .or_else(|| e.body_md.clone());

        tx.execute(
            "INSERT INTO entities
               (id, universe_id, type_id, name, summary, properties, body_json, body_md, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                     COALESCE(?9, datetime('now')), COALESCE(?10, datetime('now')))",
            params![
                new_eid,
                universe_id,
                new_tid,
                label,
                clean(&e.summary),
                Value::Object(props).to_string(),
                doc.as_ref().map(|d| d.to_string()),
                body_md,
                clean(&e.created_at),
                clean(&e.updated_at)
            ],
        )?;
        new_entity_ids.push(new_eid);
    }

    let mut imported_relations = 0usize;
    for r in &data.relations {
        let (Some(rt), Some(src), Some(dst)) = (
            rel_type_map.get(&r.type_id),
            entity_map.get(&r.source_id),
            entity_map.get(&r.target_id),
        ) else {
            warnings.add("uma relação foi ignorada: tipo ou entidade ausente".to_string());
            continue;
        };
        if src == dst {
            warnings.add("uma relação de uma entidade com ela mesma foi ignorada".to_string());
            continue;
        }
        // Só mantém property_key se a propriedade existir no tipo da origem.
        let property_key = r.property_key.as_deref().filter(|k| {
            entity_type_of
                .get(r.source_id.as_str())
                .and_then(|tid| types_by_old.get(tid))
                .map(|t| {
                    t.properties
                        .iter()
                        .any(|p| p.key == *k && p.data_type == "entity_ref")
                })
                .unwrap_or(false)
        });
        tx.execute(
            "INSERT INTO relations
               (id, universe_id, type_id, source_id, target_id, description, start_date, end_date, property_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                new_id(),
                universe_id,
                rt,
                src,
                dst,
                clean(&r.description),
                clean(&r.start_date),
                clean(&r.end_date),
                property_key
            ],
        )?;
        imported_relations += 1;
    }

    // Índice de pesquisa e menções (precisam de todas as entidades já gravadas).
    for id in &new_entity_ids {
        let ent = entities::get(&tx, id)?;
        entities::reindex(&tx, &ent)?;
        entities::sync_mentions(&tx, &ent)?;
    }

    tx.commit()?;
    Ok(ImportSummary {
        universe_id,
        universe_name,
        entity_types: data.entity_types.len(),
        relation_types: data.relation_types.len(),
        entities: data.entities.len(),
        relations: imported_relations,
        warnings: warnings.finish(),
    })
}

// ---------- exportação Markdown ----------

/// Nome seguro para arquivo/pasta (e para links [[...]] do Obsidian).
fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|#^[]".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    let cleaned: String = cleaned
        .trim()
        .trim_matches('.')
        .trim()
        .chars()
        .take(120)
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "sem-nome".to_string()
    } else {
        cleaned
    }
}

/// Devolve `base` (ou `base 2`, `base 3`…) sem repetir nomes já usados (sem diferenciar maiúsculas).
fn unique_name(base: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = base.to_string();
    let mut n = 2;
    while used.contains(&candidate.to_lowercase()) {
        candidate = format!("{base} {n}");
        n += 1;
    }
    used.insert(candidate.to_lowercase());
    candidate
}

fn push_kv(out: &mut String, key: &str, value: &Value) {
    // JSON é YAML válido: strings ficam entre aspas e escapadas corretamente.
    out.push_str(&format!("{key}: {}\n", value));
}

fn render_entity(
    conn: &Connection,
    e: &Entity,
    t: Option<&EntityType>,
    stems: &HashMap<&str, String>,
) -> Result<String> {
    let mut out = String::from("---\n");
    push_kv(&mut out, "uwb_id", &json!(e.id));
    push_kv(
        &mut out,
        "uwb_type",
        &json!(t.map(|t| t.name.as_str()).unwrap_or("")),
    );
    push_kv(&mut out, "uwb_created", &json!(e.created_at));
    push_kv(&mut out, "uwb_updated", &json!(e.updated_at));
    if let Some(t) = t {
        for def in &t.properties {
            let Some(v) = e.properties.get(&def.key) else {
                continue;
            };
            let v = if def.data_type == "entity_ref" {
                match v.as_str().and_then(|id| stems.get(id)) {
                    Some(stem) => json!(format!("[[{stem}]]")),
                    None => v.clone(),
                }
            } else {
                v.clone()
            };
            push_kv(&mut out, &def.key, &v);
        }
    }
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", e.name));

    if let Some(s) = e.summary.as_deref().filter(|s| !s.trim().is_empty()) {
        out.push_str(&format!("> {}\n\n", s.replace('\n', "\n> ")));
    }

    // Menções usam o nome de arquivo final, para o link nunca ficar ambíguo.
    let body = match &e.body_json {
        Some(doc) => {
            let mut doc = doc.clone();
            for id in richtext::mention_ids(&doc) {
                if let Some(stem) = stems.get(id.as_str()) {
                    richtext::rename_mention(&mut doc, &id, stem);
                }
            }
            richtext::to_markdown(&doc)
        }
        None => e.body_md.clone().unwrap_or_default(),
    };
    if !body.trim().is_empty() {
        out.push_str(body.trim());
        out.push_str("\n\n");
    }

    let rels: Vec<_> = relations::list_for_entity(conn, &e.id)?
        .into_iter()
        .filter(|r| r.property_key.is_none()) // as de propriedade já estão no frontmatter
        .collect();
    if !rels.is_empty() {
        out.push_str("## Relações\n\n");
        for r in rels {
            let other = stems
                .get(r.other.id.as_str())
                .cloned()
                .unwrap_or_else(|| r.other.name.clone());
            let arrow = match r.direction.as_str() {
                "incoming" => "←",
                "both" => "↔",
                _ => "→",
            };
            let mut line = format!("- {arrow} {} [[{other}]]", r.label);
            if r.start_date.is_some() || r.end_date.is_some() {
                line.push_str(&format!(
                    " ({} → {})",
                    r.start_date.as_deref().unwrap_or("…"),
                    r.end_date.as_deref().unwrap_or("…")
                ));
            }
            if let Some(d) = r.description.as_deref().filter(|d| !d.trim().is_empty()) {
                line.push_str(&format!(" — {d}"));
            }
            line.push('\n');
            out.push_str(&line);
        }
    }
    Ok(out)
}

/// Escreve <parent>/<Universo>/<Tipo>/<Entidade>.md (+ _Índice.md).
pub fn export_markdown(conn: &Connection, universe_id: &str, parent: &Path) -> Result<ExportResult> {
    let universe = find_universe(conn, universe_id)?;
    let types = entity_types::list(conn, universe_id)?;
    let ents = entities::list(conn, universe_id, None)?;
    let type_by_id: HashMap<&str, &EntityType> =
        types.iter().map(|t| (t.id.as_str(), t)).collect();

    let mut folder_names: HashSet<String> = HashSet::new();
    let mut folder_of: HashMap<&str, String> = HashMap::new();
    for t in &types {
        let name = unique_name(&safe_name(&t.name), &mut folder_names);
        folder_of.insert(t.id.as_str(), name);
    }

    // Nomes de arquivo únicos no universo todo: os links [[...]] não podem ser ambíguos.
    let mut used: HashSet<String> = HashSet::new();
    used.insert("_índice".to_string());
    let mut stem_of: HashMap<&str, String> = HashMap::new();
    for e in &ents {
        let base = safe_name(&e.name);
        let mut stem = base.clone();
        if used.contains(&stem.to_lowercase()) {
            let tname = type_by_id
                .get(e.type_id.as_str())
                .map(|t| t.name.as_str())
                .unwrap_or("");
            stem = format!("{base} ({})", safe_name(tname));
        }
        let stem = unique_name(&stem, &mut used);
        stem_of.insert(e.id.as_str(), stem);
    }

    fs::create_dir_all(parent)?;
    let dir_base = safe_name(&universe.name);
    let mut dir = parent.join(&dir_base);
    let mut n = 2;
    while dir.exists() {
        dir = parent.join(format!("{dir_base} ({n})"));
        n += 1;
    }
    fs::create_dir_all(&dir)?;

    let written = (|| -> Result<()> {
        for e in &ents {
            let folder = folder_of
                .get(e.type_id.as_str())
                .cloned()
                .unwrap_or_else(|| "Sem tipo".to_string());
            fs::create_dir_all(dir.join(&folder))?;
            let text = render_entity(
                conn,
                e,
                type_by_id.get(e.type_id.as_str()).copied(),
                &stem_of,
            )?;
            let file = dir
                .join(&folder)
                .join(format!("{}.md", stem_of[e.id.as_str()]));
            fs::write(file, text)?;
        }

        let mut index = format!("# {}\n\n", universe.name);
        if let Some(d) = universe
            .description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
        {
            index.push_str(d);
            index.push_str("\n\n");
        }
        for t in &types {
            let items: Vec<&Entity> = ents.iter().filter(|e| e.type_id == t.id).collect();
            if items.is_empty() {
                continue;
            }
            index.push_str(&format!("## {}\n\n", t.name));
            for e in items {
                index.push_str(&format!("- [[{}]]\n", stem_of[e.id.as_str()]));
            }
            index.push('\n');
        }
        fs::write(dir.join("_Índice.md"), index)?;
        Ok(())
    })();

    if let Err(e) = written {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(ExportResult {
        path: dir.to_string_lossy().into_owned(),
        entities: ents.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::search;
    use std::path::Path;

    fn setup() -> (Connection, String) {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let u = universes::create(&conn, "U", Some("descrição")).unwrap();
        let t = entity_types::create(
            &conn,
            serde_json::from_value(json!({
                "universeId": u.id, "name": "Pessoa",
                "properties": [
                    {"key": "idade", "label": "Idade", "dataType": "number"},
                    {"key": "amigo", "label": "Amigo", "dataType": "entity_ref"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let elena = entities::create(
            &conn,
            serde_json::from_value(json!({
                "universeId": u.id, "typeId": t.id, "name": "Elena",
                "properties": {"idade": 41}
            }))
            .unwrap(),
        )
        .unwrap();
        let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[
            {"type":"text","text":"Aliada de "},
            {"type":"mention","attrs":{"id": elena.id, "label":"Elena"}}]}]});
        let aurora = entities::create(
            &conn,
            serde_json::from_value(json!({
                "universeId": u.id, "typeId": t.id, "name": "Aurora",
                "properties": {"amigo": elena.id}, "bodyJson": doc
            }))
            .unwrap(),
        )
        .unwrap();
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
                "universeId": u.id, "typeId": rt.id,
                "sourceId": elena.id, "targetId": aurora.id
            }))
            .unwrap(),
        )
        .unwrap();
        (conn, u.id)
    }

    #[test]
    fn exportar_e_importar_remapeia_ids() {
        let (conn, uid) = setup();
        let data = build_export(&conn, &uid).unwrap();
        // Passa pelo texto JSON, como acontece com o arquivo de verdade.
        let data: UniverseExport =
            serde_json::from_str(&serde_json::to_string(&data).unwrap()).unwrap();

        let s = import_universe(&conn, data, None).unwrap();
        assert_eq!(s.universe_name, "U (importado)");
        assert_eq!(
            (s.entity_types, s.entities, s.relation_types, s.relations),
            (1, 2, 2, 2)
        );
        assert!(s.warnings.is_empty(), "{:?}", s.warnings);

        let new = entities::list(&conn, &s.universe_id, None).unwrap();
        assert_eq!(new.len(), 2);
        let elena = new.iter().find(|e| e.name == "Elena").unwrap();
        let aurora = new.iter().find(|e| e.name == "Aurora").unwrap();

        assert_eq!(aurora.properties["amigo"], json!(elena.id));
        assert_eq!(aurora.body_md.as_deref(), Some("Aliada de [[Elena]]"));
        assert_eq!(
            richtext::mention_ids(aurora.body_json.as_ref().unwrap()),
            vec![elena.id.clone()]
        );
        let links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mentions WHERE source_id = ?1 AND target_id = ?2",
                params![aurora.id, elena.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(links, 1);

        // O universo original continua intacto.
        assert_eq!(entities::list(&conn, &uid, None).unwrap().len(), 2);

        // A pesquisa funciona no universo importado.
        let hits = search::search(
            &conn,
            serde_json::from_value(json!({"universeId": s.universe_id, "text": "aur"})).unwrap(),
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn importacao_invalida_nao_deixa_rastro() {
        let (conn, uid) = setup();
        let mut data = build_export(&conn, &uid).unwrap();
        data.entities[0].type_id = "inexistente".into();
        assert!(import_universe(&conn, data, None).is_err());
        assert_eq!(universes::list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn referencias_pendentes_viram_avisos() {
        let (conn, uid) = setup();
        let mut data = build_export(&conn, &uid).unwrap();
        data.entities.retain(|e| e.name != "Elena");

        let s = import_universe(&conn, data, Some("Cópia")).unwrap();
        assert_eq!(s.universe_name, "Cópia");
        assert_eq!(s.entities, 1);
        assert_eq!(s.relations, 0);
        assert!(!s.warnings.is_empty());

        let list = entities::list(&conn, &s.universe_id, None).unwrap();
        let aurora = &list[0];
        assert_eq!(aurora.body_md.as_deref(), Some("Aliada de Elena"));
        assert!(aurora.properties.get("amigo").is_none());
    }

    #[test]
    fn markdown_gera_arquivos_com_links_e_propriedades() {
        let (conn, uid) = setup();
        let tmp = std::env::temp_dir().join(format!("uwb-test-{}", new_id()));
        let r = export_markdown(&conn, &uid, &tmp).unwrap();
        assert_eq!(r.entities, 2);

        let dir = PathBuf::from(&r.path);
        let aurora = fs::read_to_string(dir.join("Pessoa").join("Aurora.md")).unwrap();
        assert!(aurora.contains("amigo: \"[[Elena]]\""));
        assert!(aurora.contains("Aliada de [[Elena]]"));
        let elena = fs::read_to_string(dir.join("Pessoa").join("Elena.md")).unwrap();
        assert!(elena.contains("idade: 41"));
        assert!(elena.contains("conhece [[Aurora]]"));
        assert!(dir.join("_Índice.md").is_file());

        fs::remove_dir_all(&tmp).ok();
    }
}