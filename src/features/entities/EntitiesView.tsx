import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { useApp } from "../../app/AppContext";
import { api, type Entity } from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ErrorBanner } from "../../shared/ui";
import { EntityForm } from "./EntityForm";

interface Props {
  onGoToTypes: () => void;
  /** Entidade a abrir (vinda da pesquisa). */
  openId: string | null;
  onOpenHandled: () => void;
}

export function EntitiesView({ onGoToTypes, openId, onOpenHandled }: Props) {
  const { universe, types, dataVersion } = useApp();
  const [entities, setEntities] = useState<Entity[]>([]);
  const [filterType, setFilterType] = useState<string | null>(null);
  const [selected, setSelected] = useState<Entity | "new" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!universe) return;
    try {
      setEntities(await api.listEntities(universe.id));
    } catch (e) {
      setError(errMsg(e));
    }
  }, [universe]);

  // Dados mudaram fora desta tela (tipos editados, universo trocado): recarrega e limpa a seleção.
  useEffect(() => {
    setSelected(null);
    setFilterType(null);
  }, [universe?.id, dataVersion]);

  useEffect(() => {
    load();
  }, [load, dataVersion]);

  // Abre a entidade pedida pela pesquisa assim que ela estiver na lista.
  useEffect(() => {
    if (!openId) return;
    const target = entities.find((e) => e.id === openId);
    if (!target) return;
    setFilterType(null);
    setSelected(target);
    onOpenHandled();
  }, [openId, entities, onOpenHandled]);

  const typeById = useMemo(() => new Map(types.map((t) => [t.id, t])), [types]);
  const visible = filterType ? entities.filter((e) => e.typeId === filterType) : entities;

  // Navegação por menção, backlink ou relação.
  const openById = useCallback(
    (id: string) => {
      const target = entities.find((e) => e.id === id);
      if (!target) return;
      if (filterType && target.typeId !== filterType) setFilterType(null);
      setSelected(target);
    },
    [entities, filterType],
  );

  if (types.length === 0) {
    return (
      <div className="mx-auto max-w-md space-y-3 p-8">
        <h2 className="text-xl font-semibold">Defina o primeiro tipo de entidade</h2>
        <p className="text-sm text-zinc-500">
          Antes de criar registros, crie um tipo (por exemplo, Personagem) com suas propriedades.
        </p>
        <Button onClick={onGoToTypes}>Ir para tipos de entidade</Button>
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Lista */}
      <section className="flex w-80 shrink-0 flex-col border-r border-zinc-200 dark:border-zinc-800">
        <div className="space-y-2 border-b border-zinc-200 p-3 dark:border-zinc-800">
          <Button className="w-full" onClick={() => setSelected("new")}>
            + Nova entidade
          </Button>
          <div className="flex flex-wrap gap-1">
            <Chip active={filterType === null} onClick={() => setFilterType(null)}>
              Todas ({entities.length})
            </Chip>
            {types.map((t) => (
              <Chip
                key={t.id}
                active={filterType === t.id}
                onClick={() => setFilterType(t.id)}
              >
                {t.icon ? `${t.icon} ` : ""}
                {t.name}
              </Chip>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          <div className="p-2">
            <ErrorBanner message={error} onClose={() => setError(null)} />
          </div>
          {visible.length === 0 && (
            <p className="p-4 text-sm text-zinc-500">Nenhuma entidade encontrada.</p>
          )}
          {visible.map((e) => {
            const t = typeById.get(e.typeId);
            const active = selected !== "new" && selected?.id === e.id;
            return (
              <button
                key={e.id}
                onClick={() => setSelected(e)}
                className={`block w-full border-b border-zinc-100 px-4 py-2 text-left dark:border-zinc-900 ${
                  active ? "bg-indigo-50 dark:bg-indigo-950" : "hover:bg-zinc-50 dark:hover:bg-zinc-900"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span
                    className="inline-block h-2 w-2 shrink-0 rounded-full"
                    style={{ background: t?.color ?? "#6366f1" }}
                  />
                  <span className="truncate text-sm font-medium">{e.name}</span>
                </div>
                <div className="truncate text-xs text-zinc-500">
                  {t?.name}
                  {e.summary ? ` · ${e.summary}` : ""}
                </div>
              </button>
            );
          })}
        </div>
      </section>

      {/* Detalhe / formulário */}
      <section className="flex-1 overflow-y-auto p-6">
        {selected === null ? (
          <p className="text-sm text-zinc-500">
            Selecione uma entidade na lista ou crie uma nova.
          </p>
        ) : (
          <EntityForm
            key={selected === "new" ? "new" : selected.id}
            entity={selected === "new" ? null : selected}
            defaultTypeId={filterType}
            entities={entities}
            onCancel={() => setSelected(null)}
            onNavigate={openById}
            onSaved={async (saved) => {
              await load();
              setSelected(saved);
            }}
            onDeleted={async () => {
              setSelected(null);
              await load();
            }}
          />
        )}
      </section>
    </div>
  );
}

function Chip({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-full px-2.5 py-0.5 text-xs ${
        active
          ? "bg-indigo-600 text-white"
          : "bg-zinc-200 text-zinc-700 hover:bg-zinc-300 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-zinc-700"
      }`}
    >
      {children}
    </button>
  );
}