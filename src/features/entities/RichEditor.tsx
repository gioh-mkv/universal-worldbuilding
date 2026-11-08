import Mention from "@tiptap/extension-mention";
import { EditorContent, useEditor, useEditorState, type JSONContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { useMemo, useRef, type MouseEvent, type ReactNode } from "react";
import { useApp } from "../../app/AppContext";
import type { Entity } from "../../shared/api";
import type { MentionItem } from "./MentionList";
import { mentionSuggestion } from "./mentionSuggestion";

interface Props {
  initial: JSONContent;
  entities: Entity[]; // candidatas a menção
  selfId?: string;
  onChange: (doc: JSONContent) => void;
  onNavigate: (id: string) => void;
}

export function RichEditor({ initial, entities, selfId, onChange, onNavigate }: Props) {
  const { types } = useApp();

  // Lista lida pelo popup no momento da digitação (sempre atual, sem recriar o editor).
  const candidates = useMemo<MentionItem[]>(
    () =>
      entities
        .filter((e) => e.id !== selfId)
        .map((e) => {
          const t = types.find((x) => x.id === e.typeId);
          return {
            id: e.id,
            label: e.name,
            typeName: t?.name ?? "",
            color: t?.color ?? "#6366f1",
          };
        }),
    [entities, selfId, types],
  );
  const candidatesRef = useRef(candidates);
  candidatesRef.current = candidates;

  const editor = useEditor({
    extensions: [
      StarterKit,
      Mention.configure({
        HTMLAttributes: { class: "mention" },
        renderText: ({ node }) => `[[${node.attrs.label ?? node.attrs.id}]]`,
        renderHTML: ({ options, node }) => [
          "span",
          options.HTMLAttributes,
          `[[${node.attrs.label ?? node.attrs.id}]]`,
        ],
        suggestion: mentionSuggestion(() => candidatesRef.current),
      }),
    ],
    content: initial,
    onUpdate: ({ editor }) => onChange(editor.getJSON()),
    editorProps: { attributes: { class: "uwb-editor" } },
  });

  const active = useEditorState({
    editor,
    selector: (ctx) => ({
      bold: ctx.editor?.isActive("bold") ?? false,
      italic: ctx.editor?.isActive("italic") ?? false,
      h2: ctx.editor?.isActive("heading", { level: 2 }) ?? false,
      bullet: ctx.editor?.isActive("bulletList") ?? false,
      ordered: ctx.editor?.isActive("orderedList") ?? false,
      quote: ctx.editor?.isActive("blockquote") ?? false,
    }),
  });

  // Ctrl/Cmd + clique em uma menção abre a entidade.
  function handleClick(e: MouseEvent<HTMLDivElement>) {
    if (!(e.ctrlKey || e.metaKey)) return;
    const el = (e.target as HTMLElement).closest('[data-type="mention"]');
    const id = el?.getAttribute("data-id");
    if (id) {
      e.preventDefault();
      onNavigate(id);
    }
  }

  if (!editor) return null;
  const chain = () => editor.chain().focus();

  return (
    <div className="rounded-md border border-zinc-300 bg-white focus-within:border-indigo-500 dark:border-zinc-700 dark:bg-zinc-900">
      <div className="flex flex-wrap gap-1 border-b border-zinc-200 p-1 dark:border-zinc-800">
        <Tb title="Negrito" active={active.bold} onClick={() => chain().toggleBold().run()}>
          <b>B</b>
        </Tb>
        <Tb title="Itálico" active={active.italic} onClick={() => chain().toggleItalic().run()}>
          <i>I</i>
        </Tb>
        <Tb
          title="Subtítulo"
          active={active.h2}
          onClick={() => chain().toggleHeading({ level: 2 }).run()}
        >
          H2
        </Tb>
        <Tb title="Lista" active={active.bullet} onClick={() => chain().toggleBulletList().run()}>
          • Lista
        </Tb>
        <Tb
          title="Lista numerada"
          active={active.ordered}
          onClick={() => chain().toggleOrderedList().run()}
        >
          1. Lista
        </Tb>
        <Tb title="Citação" active={active.quote} onClick={() => chain().toggleBlockquote().run()}>
          ❝
        </Tb>
      </div>
      <div className="px-3 py-2" onClick={handleClick}>
        <EditorContent editor={editor} />
      </div>
    </div>
  );
}

function Tb({
  active,
  onClick,
  title,
  children,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      title={title}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
      className={`rounded px-2 py-1 text-sm ${
        active
          ? "bg-indigo-600 text-white"
          : "text-zinc-700 hover:bg-zinc-200 dark:text-zinc-300 dark:hover:bg-zinc-800"
      }`}
    >
      {children}
    </button>
  );
}