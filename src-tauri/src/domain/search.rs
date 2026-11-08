use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// Delimitadores de destaque (área de uso privado do Unicode).
/// O frontend os converte em <mark>; nunca vai HTML do conteúdo do usuário.
const MARK_START: char = '\u{E000}';
const MARK_END: char = '\u{E001}';

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationFilter {
    pub entity_id: String,
    pub relation_type_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PropertyFilter {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub universe_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub type_ids: Vec<String>,
    /// A entidade precisa estar relacionada (em qualquer direção) a cada uma destas.
    #[serde(default)]
    pub related_to: Vec<RelationFilter>,
    /// O documento da entidade precisa mencionar cada uma destas.
    #[serde(default)]
    pub mentioning: Vec<String>,
    /// Igualdade de texto (sem diferenciar maiúsculas) sobre propriedades.
    #[serde(default)]
    pub property_filters: Vec<PropertyFilter>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub type_id: String,
    pub summary: Option<String>,
    /// Nome com trechos marcados (só quando o nome casou com a busca).
    pub name_hl: Option<String>,
    /// Trecho do documento com marcações (só quando o corpo casou).
    pub snippet: Option<String>,
}

/// Transforma o texto digitado em uma consulta FTS5 segura:
/// cada palavra vira um prefixo entre aspas ("aur"*), todas combinadas com AND.
fn fts_query(text: &str) -> Option<String> {
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

fn bind(params: &mut Vec<Value>, v: impl Into<Value>) -> String {
    params.push(v.into());
    format!("?{}", params.len())
}

/// Cláusulas dos filtros, todas referindo a tabela `entities` com o alias `e`.
fn filter_clauses(q: &SearchQuery, params: &mut Vec<Value>) -> Result<Vec<String>> {
    let mut out = Vec::new();

    if !q.type_ids.is_empty() {
        let ph: Vec<String> = q
            .type_ids
            .iter()
            .map(|t| bind(params, t.clone()))
            .collect();
        out.push(format!("e.type_id IN ({})", ph.join(",")));
    }

    for f in &q.related_to {
        let a = bind(params, f.entity_id.clone());
        let mut clause = format!(
            "EXISTS (SELECT 1 FROM relations r WHERE \
             ((r.source_id = e.id AND r.target_id = {a}) OR (r.target_id = e.id AND r.source_id = {a}))"
        );
        if let Some(t) = &f.relation_type_id {
            let tb = bind(params, t.clone());
            clause.push_str(&format!(" AND r.type_id = {tb}"));
        }
        clause.push(')');
        out.push(clause);
    }

    for id in &q.mentioning {
        let a = bind(params, id.clone());
        out.push(format!(
            "EXISTS (SELECT 1 FROM mentions m WHERE m.source_id = e.id AND m.target_id = {a})"
        ));
    }

    for p in &q.property_filters {
        let key = p.key.trim();
        if key.is_empty() || key.contains('"') {
            return Err(AppError::msg("chave de propriedade inválida"));
        }
        let path = bind(params, format!("$.\"{key}\""));
        let val = bind(params, p.value.trim().to_string());
        out.push(format!(
            "CAST(json_extract(e.properties, {path}) AS TEXT) = {val} COLLATE NOCASE"
        ));
    }

    Ok(out)
}

pub fn search(conn: &Connection, q: SearchQuery) -> Result<Vec<SearchHit>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200) as i64;
    let ms = MARK_START;
    let me = MARK_END;

    let mut params: Vec<Value> = Vec::new();
    let universe = bind(&mut params, q.universe_id.clone());

    let sql = if let Some(fts) = fts_query(&q.text) {
        let m = bind(&mut params, fts);
        let mut conds = vec![
            format!("search_index MATCH {m}"),
            format!("search_index.universe_id = {universe}"),
        ];
        conds.extend(filter_clauses(&q, &mut params)?);
        let lim = bind(&mut params, limit);
        format!(
            "SELECT e.id, e.name, e.type_id, e.summary,
                    highlight(search_index, 2, '{ms}', '{me}'),
                    snippet(search_index, 4, '{ms}', '{me}', '…', 14)
             FROM search_index
             JOIN entities e ON e.id = search_index.entity_id
             WHERE {}
             ORDER BY bm25(search_index, 0.0, 0.0, 10.0, 5.0, 1.0, 2.0)
             LIMIT {lim}",
            conds.join(" AND ")
        )
    } else {
        let mut conds = vec![format!("e.universe_id = {universe}")];
        conds.extend(filter_clauses(&q, &mut params)?);
        let lim = bind(&mut params, limit);
        format!(
            "SELECT e.id, e.name, e.type_id, e.summary, NULL, NULL
             FROM entities e
             WHERE {}
             ORDER BY e.name COLLATE NOCASE
             LIMIT {lim}",
            conds.join(" AND ")
        )
    };

    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map(params_from_iter(params.iter()), |r| {
        let name_hl: Option<String> = r.get(4)?;
        let snippet: Option<String> = r.get(5)?;
        Ok(SearchHit {
            id: r.get(0)?,
            name: r.get(1)?,
            type_id: r.get(2)?,
            summary: r.get(3)?,
            // snippet()/highlight() devolvem o início do texto quando a coluna não casou.
            name_hl: name_hl.filter(|s| s.contains(MARK_START)),
            snippet: snippet.filter(|s| s.contains(MARK_START)),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{self, NewEntity};
    use crate::domain::{entity_types, universes};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn consulta_fts_segura() {
        assert_eq!(fts_query("Elena  V."), Some("\"Elena\"* \"V\"*".to_string()));
        assert_eq!(fts_query("Jean-Luc"), Some("\"Jean\"* \"Luc\"*".to_string()));
        assert_eq!(fts_query("\"*()"), None);
    }

    #[test]
    fn busca_por_prefixo_sem_acento_e_filtros() {
        let conn = crate::db::open(Path::new(":memory:")).unwrap();
        let u = universes::create(&conn, "U", None).unwrap();
        let t = entity_types::create(
            &conn,
            serde_json::from_value(json!({"universeId": u.id, "name": "Cidade"})).unwrap(),
        )
        .unwrap();
        for (name, summary) in [("Aurora", "Capital"), ("José", "Viajante"), ("Zenith", "Porto")] {
            let new: NewEntity = serde_json::from_value(json!({
                "universeId": u.id, "typeId": t.id, "name": name, "summary": summary
            }))
            .unwrap();
            entities::create(&conn, new).unwrap();
        }

        let q = |text: &str| SearchQuery {
            universe_id: u.id.clone(),
            text: text.to_string(),
            type_ids: vec![],
            related_to: vec![],
            mentioning: vec![],
            property_filters: vec![],
            limit: None,
        };

        let hits = search(&conn, q("aur")).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Aurora");
        assert!(hits[0].name_hl.is_some());

        let hits = search(&conn, q("jose")).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "José");

        // Sem texto: lista tudo, ordenado por nome.
        let all = search(&conn, q("")).unwrap();
        assert_eq!(all.len(), 3);

        // Filtro de tipo inexistente: nada.
        let mut none = q("aur");
        none.type_ids = vec!["inexistente".to_string()];
        assert!(search(&conn, none).unwrap().is_empty());
    }
}