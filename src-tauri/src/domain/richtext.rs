//! Utilidades para documentos TipTap (JSON): extração de menções e conversão para Markdown.

use std::collections::BTreeSet;

use serde_json::Value;

fn kind(n: &Value) -> &str {
    n.get("type").and_then(Value::as_str).unwrap_or("")
}

fn kids(n: &Value) -> &[Value] {
    n.get("content")
        .and_then(Value::as_array)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// IDs de todas as entidades mencionadas no documento (sem repetição).
pub fn mention_ids(doc: &Value) -> Vec<String> {
    let mut out = BTreeSet::new();
    collect(doc, &mut out);
    out.into_iter().collect()
}

fn collect(node: &Value, out: &mut BTreeSet<String>) {
    if kind(node) == "mention" {
        if let Some(id) = node
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(Value::as_str)
        {
            out.insert(id.to_string());
        }
    }
    for c in kids(node) {
        collect(c, out);
    }
}

/// Atualiza o rótulo de todas as menções à entidade `id`. Devolve true se algo mudou.
pub fn rename_mention(node: &mut Value, id: &str, label: &str) -> bool {
    let mut changed = false;
    if node.get("type").and_then(Value::as_str) == Some("mention")
        && node.pointer("/attrs/id").and_then(Value::as_str) == Some(id)
        && node.pointer("/attrs/label").and_then(Value::as_str) != Some(label)
    {
        if let Some(attrs) = node.get_mut("attrs").and_then(Value::as_object_mut) {
            attrs.insert("label".to_string(), Value::String(label.to_string()));
            changed = true;
        }
    }
    if let Some(children) = node.get_mut("content").and_then(Value::as_array_mut) {
        for c in children.iter_mut() {
            changed |= rename_mention(c, id, label);
        }
    }
    changed
}

/// Troca as menções à entidade `id` por texto simples (usado ao excluí-la).
/// Devolve true se algo mudou.
pub fn unlink_mention(node: &mut Value, id: &str) -> bool {
    let mut changed = false;
    if let Some(children) = node.get_mut("content").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            let is_target = child.get("type").and_then(Value::as_str) == Some("mention")
                && child.pointer("/attrs/id").and_then(Value::as_str) == Some(id);
            if is_target {
                let label = mention_label(child);
                *child = serde_json::json!({ "type": "text", "text": label });
                changed = true;
            } else {
                changed |= unlink_mention(child, id);
            }
        }
    }
    changed
}

fn mention_label(n: &Value) -> String {
    let attrs = n.get("attrs");
    attrs
        .and_then(|a| a.get("label"))
        .and_then(Value::as_str)
        .or_else(|| attrs.and_then(|a| a.get("id")).and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

/// Converte o documento para Markdown. Menções viram `[[Nome]]`.
pub fn to_markdown(doc: &Value) -> String {
    block(doc).trim().to_string()
}

fn blocks(nodes: &[Value]) -> String {
    nodes
        .iter()
        .map(block)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn prefix_lines(text: &str, first: &str, rest: &str) -> String {
    if text.is_empty() {
        return first.trim_end().to_string();
    }
    text.lines()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                format!("{first}{l}")
            } else if l.is_empty() {
                rest.trim_end().to_string()
            } else {
                format!("{rest}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn block(node: &Value) -> String {
    let children = kids(node);
    match kind(node) {
        "paragraph" => inline(children),
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .clamp(1, 6) as usize;
            format!("{} {}", "#".repeat(level), inline(children))
        }
        "blockquote" => prefix_lines(&blocks(children), "> ", "> "),
        "bulletList" => children
            .iter()
            .map(|li| prefix_lines(&blocks(kids(li)), "- ", "  "))
            .collect::<Vec<_>>()
            .join("\n"),
        "orderedList" => children
            .iter()
            .enumerate()
            .map(|(i, li)| {
                let marker = format!("{}. ", i + 1);
                let pad = " ".repeat(marker.len());
                prefix_lines(&blocks(kids(li)), &marker, &pad)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        "codeBlock" => {
            let lang = node
                .get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let code: String = children
                .iter()
                .filter_map(|c| c.get("text").and_then(Value::as_str))
                .collect();
            format!("```{lang}\n{code}\n```")
        }
        "horizontalRule" => "---".to_string(),
        "doc" => blocks(children),
        _ => inline(children),
    }
}

fn inline(nodes: &[Value]) -> String {
    let mut out = String::new();
    for n in nodes {
        match kind(n) {
            "text" => {
                let mut t = n
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if let Some(marks) = n.get("marks").and_then(Value::as_array) {
                    for m in marks {
                        t = match kind(m) {
                            "code" => format!("`{t}`"),
                            "bold" => format!("**{t}**"),
                            "italic" => format!("*{t}*"),
                            "strike" => format!("~~{t}~~"),
                            "link" => {
                                let href = m
                                    .get("attrs")
                                    .and_then(|a| a.get("href"))
                                    .and_then(Value::as_str)
                                    .unwrap_or("");
                                format!("[{t}]({href})")
                            }
                            _ => t,
                        };
                    }
                }
                out.push_str(&t);
            }
            "hardBreak" => out.push_str("  \n"),
            "mention" => out.push_str(&format!("[[{}]]", mention_label(n))),
            _ => out.push_str(&inline(kids(n))),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({"type":"doc","content":[
          {"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"História"}]},
          {"type":"paragraph","content":[
            {"type":"text","text":"Fundada por "},
            {"type":"mention","attrs":{"id":"abc","label":"Elena Voss"}},
            {"type":"text","text":"."}]},
          {"type":"bulletList","content":[
            {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"um"}]}]},
            {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"dois"}]}]}]}
        ]})
    }

    #[test]
    fn markdown_com_mencao_e_lista() {
        assert_eq!(
            to_markdown(&sample()),
            "## História\n\nFundada por [[Elena Voss]].\n\n- um\n- dois"
        );
    }

    #[test]
    fn extrai_ids_de_mencoes() {
        assert_eq!(mention_ids(&sample()), vec!["abc".to_string()]);
    }

    #[test]
    fn renomeia_mencoes() {
        let mut doc = sample();
        assert!(rename_mention(&mut doc, "abc", "Elena V."));
        assert!(to_markdown(&doc).contains("[[Elena V.]]"));
        // Já está com o rótulo novo: nada muda.
        assert!(!rename_mention(&mut doc, "abc", "Elena V."));
        // Outro id: nada muda.
        assert!(!rename_mention(&mut doc, "outro", "X"));
    }

    #[test]
    fn desfaz_mencoes_em_texto() {
        let mut doc = sample();
        assert!(unlink_mention(&mut doc, "abc"));
        assert!(mention_ids(&doc).is_empty());
        assert!(to_markdown(&doc).contains("Fundada por Elena Voss."));
        assert!(!unlink_mention(&mut doc, "abc"));
    }
}