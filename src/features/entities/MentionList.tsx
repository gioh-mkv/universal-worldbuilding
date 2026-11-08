import { forwardRef, useEffect, useImperativeHandle, useState } from "react";

export interface MentionItem {
  id: string;
  label: string;
  typeName: string;
  color: string;
}

export interface MentionListProps {
  items: MentionItem[];
  command: (attrs: { id: string; label: string }) => void;
}

export interface MentionListHandle {
  onKeyDown: (event: KeyboardEvent) => boolean;
}

export const MentionList = forwardRef<MentionListHandle, MentionListProps>(
  ({ items, command }, ref) => {
    const [index, setIndex] = useState(0);

    useEffect(() => setIndex(0), [items]);

    const pick = (i: number) => {
      const item = items[i];
      if (item) command({ id: item.id, label: item.label });
    };

    useImperativeHandle(ref, () => ({
      onKeyDown: (event) => {
        if (items.length === 0) return false;
        if (event.key === "ArrowUp") {
          setIndex((i) => (i + items.length - 1) % items.length);
          return true;
        }
        if (event.key === "ArrowDown") {
          setIndex((i) => (i + 1) % items.length);
          return true;
        }
        if (event.key === "Enter" || event.key === "Tab") {
          pick(index);
          return true;
        }
        return false;
      },
    }));

    return (
      <div className="max-h-64 w-64 overflow-y-auto rounded-md border border-zinc-200 bg-white p-1 shadow-lg dark:border-zinc-700 dark:bg-zinc-900">
        {items.length === 0 ? (
          <div className="px-3 py-2 text-xs text-zinc-500">Nenhuma entidade encontrada</div>
        ) : (
          items.map((it, i) => (
            <button
              key={it.id}
              type="button"
              onMouseDown={(e) => e.preventDefault()} // mantém o foco no editor
              onClick={() => pick(i)}
              className={`flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm ${
                i === index
                  ? "bg-indigo-600 text-white"
                  : "text-zinc-800 hover:bg-zinc-100 dark:text-zinc-100 dark:hover:bg-zinc-800"
              }`}
            >
              <span
                className="h-2 w-2 shrink-0 rounded-full"
                style={{ background: it.color }}
              />
              <span className="truncate">{it.label}</span>
              <span className="ml-auto text-xs opacity-60">{it.typeName}</span>
            </button>
          ))
        )}
      </div>
    );
  },
);
MentionList.displayName = "MentionList";