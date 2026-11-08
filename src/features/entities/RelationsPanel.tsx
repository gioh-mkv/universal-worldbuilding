import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { useApp } from "../../app/AppContext";
import {
  api,
  type Directionality,
  type Entity,
  type RelationType,
  type RelationView,
} from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ErrorBanner, Field, inputCls } from "../../shared/ui";

const NEW_TYPE = "__new__";
const ARROW = { outgoing: "→", incoming: "←", both: "↔" } as const;

const norm = (s: string) =>
  s.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();

interface Props {
  entity: Entity;
  entities: Entity[];
  onNavigate: (id: string) => void;
}

export function RelationsPanel({ entity, entities, onNavigate }: Props) {
  const { universe, types } = useApp();
  const [relations, setRelations] = useState<RelationView[]>([]);
  const [relTypes, setRelTypes] = useState<RelationType[]>([]);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!universe) return;
    try {
      const [rels, rts] = await Promise.all([
        api.listEntityRelations(entity.id),
        api.listRelationTypes(universe.id),
      ]);
      setRelations(rels);
      setRelTypes(rts);
    } catch (e) {
      setError(errMsg(e));
    }
  }, [entity.id, universe]);

  // Recarrega ao abrir e a cada salvamento (relações de propriedades mudam).
  useEffect(() => {
    reload();
  }, [reload, entity.updatedAt]);

  async function remove(id: string) {
    try {
      await api.deleteRelation(id);
      await reload();
    } catch (e) {
      setError(errMsg(e));
    }
  }

  const typeColor = (id: string) => types.find((t) => t.id === id)?.color ?? "#6366f1";

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium uppercase tracking-wide text-zinc-500">Relações</h3>
        {!adding && (
          <Button variant="secondary" onClick={() => setAdding(true)}>
            + Nova relação
          </Button>
        )}
      </div>

      <ErrorBanner message={error} onClose={() => setError(null)} />

      {adding && (
        <AddRelationForm
          entity={entity}
          entities={entities}
          relTypes={relTypes}
          onCancel={() => setAdding(false)}
          onCreated={async () => {
            setAdding(false);
            await reload();
          }}
        />
      )}

      {relations.length === 0 ? (
        <p className="text-xs text-zinc-500">Nenhuma relação.</p>
      ) : (
        <ul className="space-y-1.5">
          {relations.map((r) => (
            <li key={r.id} className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5 text-sm">
              <span className="text-xs text-zinc-500">
                {ARROW[r.direction]} {r.label}
              </span>
              <button
                type="button"
                onClick={() => onNavigate(r.other.id)}
                className="flex items-center gap-1.5 text-indigo-600 hover:underline dark:text-indigo-400"
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ background: typeColor(r.other.typeId) }}
                />
                {r.other.name}
              </button>
              {(r.startDate || r.endDate) && (
                <span className="text-xs text-zinc-500">
                  ({r.startDate ?? "…"} → {r.endDate ?? "…"})
                </span>
              )}
              {r.description && <span className="text-xs text-zinc-500">— {r.description}</span>}
              {r.propertyKey ? (
                <span className="rounded bg-zinc-200 px-1.5 text-[10px] text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400">
                  via propriedade
                </span>
              ) : (
                <button
                  type="button"
                  onClick={() => remove(r.id)}
                  title="Remover relação"
                  className="text-xs text-zinc-400 hover:text-rose-500"
                >
                  ✕
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function AddRelationForm({
  entity,
  entities,
  relTypes,
  onCreated,
  onCancel,
}: {
  entity: Entity;
  entities: Entity[];
  relTypes: RelationType[];
  onCreated: () => Promise<void>;
  onCancel: () => void;
}) {
  const { universe, types } = useApp();
  const [typeChoice, setTypeChoice] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [newInverse, setNewInverse] = useState("");
  const [newDir, setNewDir] = useState<Directionality>("directed");
  const [direction, setDirection] = useState<"out" | "in">("out");
  const [filter, setFilter] = useState("");
  const [targetId, setTargetId] = useState("");
  const [description, setDescription] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const choice = typeChoice ?? relTypes[0]?.id ?? NEW_TYPE;
  const creatingType = choice === NEW_TYPE;

  const candidates = useMemo(
    () =>
      entities
        .filter((e) => e.id !== entity.id && norm(e.name).includes(norm(filter)))
        .slice(0, 50),
    [entities, entity.id, filter],
  );

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!universe || !targetId) return;
    setBusy(true);
    setError(null);
    try {
      let typeId = choice;
      if (creatingType) {
        const t = await api.createRelationType({
          universeId: universe.id,
          name: newName,
          inverseName: newInverse.trim() || undefined,
          directionality: newDir,
        });
        typeId = t.id;
      }
      await api.createRelation({
        universeId: universe.id,
        typeId,
        sourceId: direction === "out" ? entity.id : targetId,
        targetId: direction === "out" ? targetId : entity.id,
        description: description.trim() || undefined,
        startDate: start.trim() || undefined,
        endDate: end.trim() || undefined,
      });
      await onCreated();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form
      onSubmit={submit}
      className="space-y-3 rounded-md border border-zinc-200 p-3 dark:border-zinc-800"
    >
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <Field label="Tipo de relação">
        <select
          className={inputCls}
          value={choice}
          onChange={(e) => setTypeChoice(e.target.value)}
        >
          {relTypes.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
              {t.inverseName ? ` / ${t.inverseName}` : ""}
            </option>
          ))}
          <option value={NEW_TYPE}>➕ Novo tipo de relação…</option>
        </select>
      </Field>

      {creatingType && (
        <div className="grid gap-2 sm:grid-cols-3">
          <input
            className={inputCls}
            placeholder="Nome (ex.: pertence a)"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          {newDir !== "bidirectional" && (
            <input
              className={inputCls}
              placeholder="Inverso (ex.: possui)"
              value={newInverse}
              onChange={(e) => setNewInverse(e.target.value)}
            />
          )}
          <select
            className={inputCls}
            value={newDir}
            onChange={(e) => setNewDir(e.target.value as Directionality)}
          >
            <option value="directed">Direcional</option>
            <option value="bidirectional">Bidirecional</option>
            <option value="hierarchical">Hierárquica (origem acima do destino)</option>
          </select>
        </div>
      )}

      <div className="flex gap-4 text-sm">
        <label className="flex items-center gap-1.5">
          <input
            type="radio"
            checked={direction === "out"}
            onChange={() => setDirection("out")}
          />
          Esta entidade → outra
        </label>
        <label className="flex items-center gap-1.5">
          <input
            type="radio"
            checked={direction === "in"}
            onChange={() => setDirection("in")}
          />
          Outra → esta entidade
        </label>
      </div>

      <Field label="Outra entidade" required>
        <input
          className={`${inputCls} mb-1`}
          placeholder="Filtrar por nome…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <select
          size={5}
          className={inputCls}
          value={targetId}
          onChange={(e) => setTargetId(e.target.value)}
        >
          {candidates.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name} · {types.find((t) => t.id === c.typeId)?.name ?? ""}
            </option>
          ))}
        </select>
      </Field>

      <div className="grid gap-2 sm:grid-cols-2">
        <Field label="Início" hint="Texto livre; calendários próprios virão depois.">
          <input
            className={inputCls}
            placeholder="Ex.: 2145"
            value={start}
            onChange={(e) => setStart(e.target.value)}
          />
        </Field>
        <Field label="Fim">
          <input
            className={inputCls}
            value={end}
            onChange={(e) => setEnd(e.target.value)}
          />
        </Field>
      </div>

      <Field label="Descrição">
        <input
          className={inputCls}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </Field>

      <div className="flex gap-2">
        <Button
          type="submit"
          disabled={busy || !targetId || (creatingType && !newName.trim())}
        >
          Criar relação
        </Button>
        <Button variant="ghost" onClick={onCancel}>
          Cancelar
        </Button>
      </div>
    </form>
  );
}