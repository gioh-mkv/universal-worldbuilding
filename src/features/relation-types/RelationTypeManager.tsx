import { useCallback, useEffect, useState, type FormEvent } from "react";
import { useApp } from "../../app/AppContext";
import { api, type Directionality, type RelationType } from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ConfirmDialog, ErrorBanner, Field, inputCls } from "../../shared/ui";

const DIR_LABEL: Record<Directionality, string> = {
  directed: "Direcional",
  bidirectional: "Bidirecional",
  hierarchical: "Hierárquica",
};

export function RelationTypeManager() {
  const { universe } = useApp();
  const [items, setItems] = useState<RelationType[]>([]);
  const [editing, setEditing] = useState<RelationType | null>(null);
  const [name, setName] = useState("");
  const [inverse, setInverse] = useState("");
  const [dir, setDir] = useState<Directionality>("directed");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [deleting, setDeleting] = useState<RelationType | null>(null);

  const load = useCallback(async () => {
    if (!universe) return;
    try {
      setItems(await api.listRelationTypes(universe.id));
    } catch (e) {
      setError(errMsg(e));
    }
  }, [universe]);

  useEffect(() => {
    load();
  }, [load]);

  function reset() {
    setEditing(null);
    setName("");
    setInverse("");
    setDir("directed");
    setError(null);
  }

  function startEdit(t: RelationType) {
    setEditing(t);
    setName(t.name);
    setInverse(t.inverseName ?? "");
    setDir(t.directionality);
    setError(null);
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!universe) return;
    setBusy(true);
    setError(null);
    try {
      if (editing) {
        await api.updateRelationType({
          id: editing.id,
          name,
          inverseName: inverse.trim() || undefined,
          directionality: dir,
        });
      } else {
        await api.createRelationType({
          universeId: universe.id,
          name,
          inverseName: inverse.trim() || undefined,
          directionality: dir,
        });
      }
      reset();
      await load();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (!deleting) return;
    const target = deleting;
    try {
      await api.deleteRelationType(target.id);
      if (editing?.id === target.id) reset();
      await load();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setDeleting(null);
    }
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto grid max-w-5xl gap-8 lg:grid-cols-2">
        <form onSubmit={submit} className="space-y-4">
          <h2 className="text-lg font-semibold">
            {editing ? `Editar: ${editing.name}` : "Novo tipo de relação"}
          </h2>
          <ErrorBanner message={error} onClose={() => setError(null)} />

          <Field label="Nome" required>
            <input
              className={inputCls}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Ex.: pertence a"
            />
          </Field>

          <Field label="Direcionalidade">
            <select
              className={inputCls}
              value={dir}
              onChange={(e) => setDir(e.target.value as Directionality)}
            >
              <option value="directed">Direcional (A → B)</option>
              <option value="bidirectional">Bidirecional (A ↔ B)</option>
              <option value="hierarchical">Hierárquica (origem acima do destino)</option>
            </select>
          </Field>

          {dir !== "bidirectional" && (
            <Field label="Nome inverso" hint="Como a relação aparece do lado do destino.">
              <input
                className={inputCls}
                value={inverse}
                onChange={(e) => setInverse(e.target.value)}
                placeholder="Ex.: possui"
              />
            </Field>
          )}

          <div className="flex gap-2">
            <Button type="submit" disabled={busy || !name.trim()}>
              {editing ? "Salvar alterações" : "Criar tipo"}
            </Button>
            {editing && (
              <Button variant="ghost" onClick={reset}>
                Cancelar edição
              </Button>
            )}
          </div>
        </form>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold">Tipos deste universo</h2>
          {items.length === 0 && (
            <p className="text-sm text-zinc-500">Nenhum tipo de relação ainda.</p>
          )}
          {items.map((t) => (
            <div
              key={t.id}
              className="flex items-start justify-between gap-2 rounded-md border border-zinc-200 p-3 dark:border-zinc-800"
            >
              <div>
                <div className="font-medium">
                  {t.name}
                  {t.inverseName && (
                    <span className="font-normal text-zinc-500"> / {t.inverseName}</span>
                  )}
                </div>
                <div className="text-xs text-zinc-500">
                  {DIR_LABEL[t.directionality]} · usada em {t.usageCount}{" "}
                  {t.usageCount === 1 ? "relação" : "relações"}
                </div>
              </div>
              <div className="flex gap-1">
                <Button variant="ghost" onClick={() => startEdit(t)}>
                  Editar
                </Button>
                <Button variant="ghost" onClick={() => setDeleting(t)}>
                  Excluir
                </Button>
              </div>
            </div>
          ))}
        </section>
      </div>

      <ConfirmDialog
        open={deleting !== null}
        danger
        title={`Excluir o tipo "${deleting?.name ?? ""}"?`}
        confirmLabel="Excluir tipo"
        message={
          <p>
            Só é possível excluir tipos que não são usados por nenhuma relação. Se houver, o app
            avisará quantas são.
          </p>
        }
        onConfirm={remove}
        onCancel={() => setDeleting(null)}
      />
    </div>
  );
}