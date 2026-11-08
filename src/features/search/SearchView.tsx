import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useApp } from "../../app/AppContext";
import {
  api,
  type Entity,
  type RelationType,
  type SearchHit,
  type SearchQuery,
} from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ErrorBanner, inputCls } from "../../shared/ui";

const MARK_START = "\uE000";
const MARK_END = "\uE001";
const LIMIT = 100;

/** Converte os delimitadores do backend em <mark> (sem usar HTML do conteúdo). */
function Highlight({ text }: { text: string }) {
  const parts: ReactNode[] = [];
  let rest = text;
  let i = 0;
  for (;;) {
    const s = rest.indexOf(MARK_START);
    const e = s < 0 ? -1 : rest.indexOf(MARK_END, s);
    if (s < 0 || e < 0) {
      parts.push(rest);
      break;
    }
    parts.push(rest.slice(0, s));
    parts.push(
      <mark
        key={i++}
        className="rounded bg-yellow-200 px-0.5 text-zinc-900 dark:bg-yellow-600/60 dark:text-zinc-50"
      >
        {rest.slice(s + 1, e)}
      </mark>,
    );
    rest = rest.slice(e + 1);
  }
  return <>{parts}</>;
}

interface RelRow { entityId: string; relationTypeId: string }
interface PropRow { key: string; value: string }

export function SearchView({ onOpen }: { onOpen: (id: string) => void }) {
  const { universe, types } = useApp();
  const [text, setText] = useState("");
  const [typeIds, setTypeIds] = useState<string[]>([]);
  const [relRows, setRelRows] = useState<RelRow[]>([]);
  const [mentionRows, setMentionRows] = useState<string[]>([]);
  const [propRows, setPropRows] = useState<PropRow[]>([]);

  const [entities, setEntities] = useState<Entity[]>([]);
  const [relTypes, setRelTypes] = useState<RelationType[]>([]);
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Foco na abertura e Ctrl+K para voltar ao campo de busca.
  useEffect(() => {
    inputRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Dados para montar os filtros.
  useEffect(() => {
    if (!universe) return;
    let alive = true;
    Promise.all([api.listEntities(universe.id), api.listRelationTypes(universe.id)])
      .then(([es, rts]) => {
        if (!alive) return;
        setEntities(es);
        setRelTypes(rts);
      })
      .catch((e) => alive && setError(errMsg(e)));
    return () => {
      alive = false;
    };
  }, [universe]);

  const typeById = useMemo(() => new Map(types.map((t) => [t.id, t])), [types]);

  // Propriedades disponíveis para filtro (uma entrada por chave).
  const propOptions = useMemo(() => {
    const m = new Map<string, string>();
    for (const t of types) for (const p of t.properties) if (!m.has(p.key)) m.set(p.key, p.label);
    return [...m.entries()].map(([key, label]) => ({ key, label }));
  }, [types]);

  const query = useMemo<SearchQuery | null>(() => {
    if (!universe) return null;
    return {
      universeId: universe.id,
      text,
      typeIds,
      relatedTo: relRows
        .filter((r) => r.entityId)
        .map((r) => ({ entityId: r.entityId, relationTypeId: r.relationTypeId || undefined })),
      mentioning: mentionRows.filter(Boolean),
      propertyFilters: propRows.filter((r) => r.key && r.value.trim()),
      limit: LIMIT,
    };
  }, [universe, text, typeIds, relRows, mentionRows, propRows]);

  // Busca com debounce; ignora respostas antigas.
  useEffect(() => {
    if (!query) return;
    let alive = true;
    const timer = setTimeout(async () => {
      setLoading(true);
      try {
        const r = await api.search(query);
        if (alive) {
          setHits(r);
          setError(null);
        }
      } catch (e) {
        if (alive) setError(errMsg(e));
      } finally {
        if (alive) setLoading(false);
      }
    }, 250);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [query]);

  const hasFilters =
    typeIds.length > 0 || relRows.length > 0 || mentionRows.length > 0 || propRows.length > 0;

  function clearFilters() {
    setTypeIds([]);
    setRelRows([]);
    setMentionRows([]);
    setPropRows([]);
  }

  const entityOptions = entities.map((e) => (
    <option key={e.id} value={e.id}>
      {e.name} · {typeById.get(e.typeId)?.name ?? ""}
    </option>
  ));

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-zinc-200 p-4 dark:border-zinc-800">
        <input
          ref={inputRef}
          className={`${inputCls} text-base`}
          placeholder="Pesquisar nomes, resumos, documentos e propriedades…  (Ctrl+K)"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
      </div>

      <div className="flex min-h-0 flex-1">
        {/* Filtros */}
        <aside className="w-72 shrink-0 space-y-5 overflow-y-auto border-r border-zinc-200 p-4 dark:border-zinc-800">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold">Filtros</h2>
            {hasFilters && (
              <Button variant="ghost" onClick={clearFilters}>
                Limpar
              </Button>
            )}
          </div>

          <FilterBlock title="Tipo">
            <div className="flex flex-wrap gap-1">
              {types.map((t) => {
                const on = typeIds.includes(t.id);
                return (
                  <button
                    key={t.id}
                    type="button"
                    onClick={() =>
                      setTypeIds((prev) => (on ? prev.filter((x) => x !== t.id) : [...prev, t.id]))
                    }
                    className={`rounded-full px-2.5 py-0.5 text-xs ${
                      on
                        ? "bg-indigo-600 text-white"
                        : "bg-zinc-200 text-zinc-700 hover:bg-zinc-300 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-zinc-700"
                    }`}
                  >
                    {t.icon ? `${t.icon} ` : ""}
                    {t.name}
                  </button>
                );
              })}
            </div>
          </FilterBlock>

          <FilterBlock
            title="Relacionada a"
            onAdd={() => setRelRows((r) => [...r, { entityId: "", relationTypeId: "" }])}
          >
            {relRows.map((row, i) => (
              <RowBox key={i} onRemove={() => setRelRows((r) => r.filter((_, j) => j !== i))}>
                <select
                  className={inputCls}
                  value={row.relationTypeId}
                  onChange={(e) =>
                    setRelRows((r) =>
                      r.map((x, j) => (j === i ? { ...x, relationTypeId: e.target.value } : x)),
                    )
                  }
                >
                  <option value="">Qualquer relação</option>
                  {relTypes.map((t) => (
                    <option key={t.id} value={t.id}>
                      {t.name}
                      {t.inverseName ? ` / ${t.inverseName}` : ""}
                    </option>
                  ))}
                </select>
                <select
                  className={inputCls}
                  value={row.entityId}
                  onChange={(e) =>
                    setRelRows((r) =>
                      r.map((x, j) => (j === i ? { ...x, entityId: e.target.value } : x)),
                    )
                  }
                >
                  <option value="">Escolha a entidade…</option>
                  {entityOptions}
                </select>
              </RowBox>
            ))}
          </FilterBlock>

          <FilterBlock title="Menciona" onAdd={() => setMentionRows((r) => [...r, ""])}>
            {mentionRows.map((id, i) => (
              <RowBox key={i} onRemove={() => setMentionRows((r) => r.filter((_, j) => j !== i))}>
                <select
                  className={inputCls}
                  value={id}
                  onChange={(e) =>
                    setMentionRows((r) => r.map((x, j) => (j === i ? e.target.value : x)))
                  }
                >
                  <option value="">Escolha a entidade…</option>
                  {entityOptions}
                </select>
              </RowBox>
            ))}
          </FilterBlock>

          <FilterBlock
            title="Propriedade"
            onAdd={() => setPropRows((r) => [...r, { key: propOptions[0]?.key ?? "", value: "" }])}
          >
            {propRows.map((row, i) => (
              <RowBox key={i} onRemove={() => setPropRows((r) => r.filter((_, j) => j !== i))}>
                <select
                  className={inputCls}
                  value={row.key}
                  onChange={(e) =>
                    setPropRows((r) => r.map((x, j) => (j === i ? { ...x, key: e.target.value } : x)))
                  }
                >
                  {propOptions.map((p) => (
                    <option key={p.key} value={p.key}>
                      {p.label}
                    </option>
                  ))}
                </select>
                <input
                  className={inputCls}
                  placeholder="é igual a…"
                  value={row.value}
                  onChange={(e) =>
                    setPropRows((r) =>
                      r.map((x, j) => (j === i ? { ...x, value: e.target.value } : x)),
                    )
                  }
                />
              </RowBox>
            ))}
          </FilterBlock>
        </aside>

        {/* Resultados */}
        <section className="min-w-0 flex-1 overflow-y-auto">
          <div className="space-y-2 p-3">
            <ErrorBanner message={error} onClose={() => setError(null)} />
            <div className="text-xs text-zinc-500">
              {loading
                ? "Pesquisando…"
                : text.trim() || hasFilters
                  ? `${hits.length} resultado${hits.length === 1 ? "" : "s"}`
                  : "Todas as entidades"}
              {hits.length >= LIMIT && ` (mostrando os primeiros ${LIMIT})`}
            </div>
          </div>

          {!loading && hits.length === 0 && (
            <p className="px-4 text-sm text-zinc-500">Nada encontrado.</p>
          )}

          {hits.map((h) => {
            const t = typeById.get(h.typeId);
            return (
              <button
                key={h.id}
                type="button"
                onClick={() => onOpen(h.id)}
                className="block w-full border-b border-zinc-100 px-4 py-3 text-left hover:bg-zinc-50 dark:border-zinc-900 dark:hover:bg-zinc-900"
              >
                <div className="flex items-center gap-2">
                  <span
                    className="inline-block h-2 w-2 shrink-0 rounded-full"
                    style={{ background: t?.color ?? "#6366f1" }}
                  />
                  <span className="font-medium">
                    {h.nameHl ? <Highlight text={h.nameHl} /> : h.name}
                  </span>
                  <span className="ml-auto text-xs text-zinc-500">
                    {t?.icon ? `${t.icon} ` : ""}
                    {t?.name}
                  </span>
                </div>
                {(h.snippet || h.summary) && (
                  <div className="mt-0.5 text-xs text-zinc-500">
                    {h.snippet ? <Highlight text={h.snippet} /> : h.summary}
                  </div>
                )}
              </button>
            );
          })}
        </section>
      </div>
    </div>
  );
}

function FilterBlock({
  title,
  onAdd,
  children,
}: {
  title: string;
  onAdd?: () => void;
  children: ReactNode;
}) {
  return (
    <section className="space-y-2">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium uppercase tracking-wide text-zinc-500">{title}</h3>
        {onAdd && (
          <button
            type="button"
            onClick={onAdd}
            className="text-xs text-indigo-600 hover:underline dark:text-indigo-400"
          >
            + adicionar
          </button>
        )}
      </div>
      {children}
    </section>
  );
}

function RowBox({ onRemove, children }: { onRemove: () => void; children: ReactNode }) {
  return (
    <div className="relative space-y-1 rounded-md border border-zinc-200 p-2 pr-7 dark:border-zinc-800">
      {children}
      <button
        type="button"
        onClick={onRemove}
        title="Remover filtro"
        className="absolute right-1.5 top-1.5 text-xs text-zinc-400 hover:text-rose-500"
      >
        ✕
      </button>
    </div>
  );
}