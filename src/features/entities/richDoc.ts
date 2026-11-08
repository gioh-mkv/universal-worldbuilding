import type { JSONContent } from "@tiptap/react";
import type { Entity } from "../../shared/api";

export const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

/** Linhas de texto → nós inline separados por quebra de linha. */
function inlineFromText(text: string): JSONContent[] {
  const out: JSONContent[] = [];
  text.split("\n").forEach((line, i) => {
    if (i > 0) out.push({ type: "hardBreak" });
    if (line) out.push({ type: "text", text: line });
  });
  return out;
}

/** Usa body_json; se só houver body_md (formato antigo), converte em parágrafos. */
export function initialDoc(entity: Entity | null): JSONContent {
  if (entity?.bodyJson) return entity.bodyJson as JSONContent;
  const md = entity?.bodyMd?.trim();
  if (md) {
    return {
      type: "doc",
      content: md.split(/\n{2,}/).map((par) => ({
        type: "paragraph",
        content: inlineFromText(par),
      })),
    };
  }
  return EMPTY_DOC;
}

/** Atualiza o rótulo de cada menção com o nome atual da entidade. */
export function refreshMentionLabels(
  node: JSONContent,
  names: Map<string, string>,
): JSONContent {
  const next: JSONContent = { ...node };
  if (node.type === "mention" && typeof node.attrs?.id === "string") {
    const label = names.get(node.attrs.id);
    if (label) next.attrs = { ...node.attrs, label };
  }
  if (node.content) {
    next.content = node.content.map((c) => refreshMentionLabels(c, names));
  }
  return next;
}