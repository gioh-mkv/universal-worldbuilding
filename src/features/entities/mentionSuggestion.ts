import { ReactRenderer } from "@tiptap/react";
import type { SuggestionKeyDownProps, SuggestionProps } from "@tiptap/suggestion";
import {
  MentionList,
  type MentionItem,
  type MentionListHandle,
  type MentionListProps,
} from "./MentionList";

type Attrs = { id: string; label: string };

const norm = (s: string) =>
  s.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();

/** Configuração do popup que abre ao digitar "[[". */
export function mentionSuggestion(getItems: () => MentionItem[]) {
  return {
    char: "[[",
    allowSpaces: true,

    items: ({ query }: { query: string }): MentionItem[] => {
      const q = norm(query.trim());
      const all = getItems();
      const hits = q ? all.filter((i) => norm(i.label).includes(q)) : [...all];
      return hits
        .sort(
          (a, b) =>
            Number(norm(b.label).startsWith(q)) - Number(norm(a.label).startsWith(q)) ||
            a.label.localeCompare(b.label),
        )
        .slice(0, 8);
    },

    render: () => {
      let component: ReactRenderer<MentionListHandle, MentionListProps> | null = null;
      let host: HTMLDivElement | null = null;

      const place = (props: SuggestionProps<MentionItem, Attrs>) => {
        const rect = props.clientRect?.();
        if (!rect || !host) return;
        host.style.left = `${rect.left}px`;
        host.style.top = `${rect.bottom + 4}px`;
      };

      const cleanup = () => {
        host?.remove();
        component?.destroy();
        host = null;
        component = null;
      };

      return {
        onStart: (props: SuggestionProps<MentionItem, Attrs>) => {
          component = new ReactRenderer(MentionList, { props, editor: props.editor });
          host = document.createElement("div");
          host.style.cssText = "position:fixed;z-index:50";
          host.appendChild(component.element);
          document.body.appendChild(host);
          place(props);
        },
        onUpdate: (props: SuggestionProps<MentionItem, Attrs>) => {
          component?.updateProps(props);
          place(props);
        },
        onKeyDown: (props: SuggestionKeyDownProps) => {
          if (props.event.key === "Escape") {
            cleanup();
            return true;
          }
          return component?.ref?.onKeyDown(props.event) ?? false;
        },
        onExit: cleanup,
      };
    },
  };
}