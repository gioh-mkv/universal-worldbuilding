import { useState, type FormEvent } from "react";
import { useApp } from "../../app/AppContext";
import { api, type DataType, type EntityType, type PropertyEdit } from "../../shared/api";
import { DATA_TYPES, dataTypeLabel } from "../../shared/dataTypes";
import { errMsg, slugKey } from "../../shared/text";
import { Button, ConfirmDialog, ErrorBanner, Field, inputCls } from "../../shared/ui";

interface DraftProp {
  id?: string; // presente = propriedade já existente
  key?: string; // idem (somente leitura)
  label: string;
  dataType: DataType;
  required: boolean;
  options: string; // separadas por vírgula (select / multiselect)
  targetTypeId: string; // entity_ref (vazio = qualquer tipo)
  baseConfig: Record<string, unknown>;
}

const emptyProp = (): DraftProp => ({
  label: "",
  dataType: "text",
  required: false,
  options: "",
  targetTypeId: "",
  baseConfig: {},
});

function draftFrom(t: EntityType): DraftProp[] {
  return t.properties.map((p) => ({
    id: p.id,
    key: p.key,
    label: p.label,
    dataType: p.dataType,
    required: p.required,
    options: Array.isArray(p.config.options) ? (p.config.options as string[]).join(", ") : "",
    targetTypeId: typeof p.config.targetTypeId === "string" ? p.config.targetTypeId : "",
    baseConfig: p.config,
  }));
}

export function EntityTypeManager() {
  const { universe, types, refreshTypes, invalidate } = useApp();
  const [editing, setEditing] = useState<EntityType | null>(null);
  const [name, setName] = useState("");
  const [icon, setIcon] = useState("");
  const [color, setColor] = useState("#6366f1");
  const [props, setProps] = useState<DraftProp[]>([emptyProp()]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [pendingRemoval, setPendingRemoval] = useState<{
    labels: string[];
    edits: PropertyEdit[];
  } | null>(null);
  const [deleting, setDeleting] = useState<EntityType | null>(null);

  function reset() {
    setEditing(null);
    setName("");
    setIcon("");
    setColor("#6366f1");
    setProps([emptyProp()]);
    setError(null);
  }

  function startEdit(t: EntityType) {
    setEditing(t);
    setName(t.name);
    setIcon(t.icon ?? "");
    setColor(t.color ?? "#6366f1");
    setProps(draftFrom(t));
    setError(null);
  }

  function patchProp(i: number, patch: Partial<DraftProp>) {
    setProps((prev) => prev.map((p, idx) => (idx === i ? { ...p, ...patch } : p)));
  }

  function moveProp(i: number, delta: -1 | 1) {
    setProps((prev) => {
      const j = i + delta;
      if (j < 0 || j >= prev.length) return prev;
      const next = [...prev];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
  }

  /** Converte os rascunhos em edições para o backend. Lança erro de validação. */
  function buildEdits(): PropertyEdit[] {
    const used = new Set(props.filter((p) => p.key).map((p) => p.key as string));
    const out: PropertyEdit[] = [];

    for (const p of props) {
      if (!p.label.trim()) {
        if (p.id) throw new Error("Uma propriedade existente está sem nome.");
        continue; // linha nova vazia é ignorada
      }
      const config: Record<string, unknown> = { ...p.baseConfig };
      if (p.dataType === "select" || p.dataType === "multiselect") {
        const options = p.options.split(",").map((s) => s.trim()).filter(Boolean);
        if (options.length === 0) throw new Error(`Informe as opções de "${p.label}".`);
        config.options = options;
      }
      if (p.dataType === "entity_ref") {
        if (p.targetTypeId) config.targetTypeId = p.targetTypeId;
        else delete config.targetTypeId;
      }

      if (p.id) {
        out.push({ id: p.id, label: p.label.trim(), config, required: p.required });
        continue;
      }
      const base = slugKey(p.label) || "campo";
      let key = base;
      let n = 2;
      while (used.has(key)) key = `${base}_${n++}`;
      used.add(key);
      out.push({
        key,
        dataType: p.dataType,
        label: p.label.trim(),
        config,
        required: p.required,
      });
    }
    return out;
  }

  async function save(edits: PropertyEdit[]) {
    if (!universe) return;
    setBusy(true);
    setError(null);
    try {
      if (editing) {
        await api.updateEntityType({
          id: editing.id,
          name,
          icon: icon.trim() || undefined,
          color,
          properties: edits,
        });
      } else {
        await api.createEntityType({
          universeId: universe.id,
          name,
          icon: icon.trim() || undefined,
          color,
          properties: edits.map((e) => ({
            key: e.key as string,
            label: e.label,
            dataType: e.dataType as DataType,
            config: e.config,
            required: e.required,
          })),
        });
      }
      await refreshTypes();
      invalidate(); // entidades já carregadas podem ter perdido valores
      reset();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    let edits: PropertyEdit[];
    try {
      edits = buildEdits();
    } catch (err) {
      setError(errMsg(err));
      return;
    }
    if (editing) {
      const kept = new Set(edits.filter((x) => x.id).map((x) => x.id));
      const removed = editing.properties.filter((p) => !kept.has(p.id)).map((p) => p.label);
      if (removed.length > 0) {
        setPendingRemoval({ labels: removed, edits });
        return;
      }
    }
    await save(edits);
  }

  async function removeType() {
    if (!deleting) return;
    const target = deleting;
    try {
      await api.deleteEntityType(target.id);
      await refreshTypes();
      invalidate();
      if (editing?.id === target.id) reset();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setDeleting(null);
    }
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto grid max-w-5xl gap-8 lg:grid-cols-2">
        {/* Formulário (novo / edição) */}
        <form onSubmit={submit} className="space-y-4">
          <h2 className="text-lg font-semibold">
            {editing ? `Editar tipo: ${editing.name}` : "Novo tipo de entidade"}
          </h2>
          <ErrorBanner message={error} onClose={() => setError(null)} />

          <div className="flex gap-3">
            <div className="flex-1">
              <Field label="Nome" required>
                <input
                  className={inputCls}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Ex.: Personagem, Planeta, Corporação…"
                />
              </Field>
            </div>
            <div className="w-20">
              <Field label="Ícone">
                <input
                  className={inputCls}
                  value={icon}
                  onChange={(e) => setIcon(e.target.value)}
                  placeholder="👤"
                  maxLength={4}
                />
              </Field>
            </div>
            <div className="w-16">
              <Field label="Cor">
                <input
                  type="color"
                  className="h-[34px] w-full cursor-pointer rounded-md border border-zinc-300 bg-white dark:border-zinc-700 dark:bg-zinc-900"
                  value={color}
                  onChange={(e) => setColor(e.target.value)}
                />
              </Field>
            </div>
          </div>

          <div className="space-y-3">
            <h3 className="text-sm font-medium text-zinc-600 dark:text-zinc-400">Propriedades</h3>
            {props.map((p, i) => (
              <div
                key={p.id ?? `new-${i}`}
                className="space-y-2 rounded-md border border-zinc-200 p-3 dark:border-zinc-800"
              >
                <div className="flex gap-2">
                  <input
                    className={inputCls}
                    placeholder="Nome da propriedade"
                    value={p.label}
                    onChange={(e) => patchProp(i, { label: e.target.value })}
                  />
                  {p.id ? (
                    <div
                      className="flex shrink-0 items-center rounded-md bg-zinc-100 px-3 text-sm text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400"
                      title="O tipo de dado não pode ser alterado. Remova e recrie a propriedade."
                    >
                      {dataTypeLabel(p.dataType)}
                    </div>
                  ) : (
                    <select
                      className={inputCls}
                      value={p.dataType}
                      onChange={(e) => patchProp(i, { dataType: e.target.value as DataType })}
                    >
                      {DATA_TYPES.map((d) => (
                        <option key={d.value} value={d.value}>
                          {d.label}
                        </option>
                      ))}
                    </select>
                  )}
                  <Button variant="ghost" onClick={() => moveProp(i, -1)} title="Subir">
                    ↑
                  </Button>
                  <Button variant="ghost" onClick={() => moveProp(i, 1)} title="Descer">
                    ↓
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => setProps((prev) => prev.filter((_, idx) => idx !== i))}
                    title="Remover propriedade"
                  >
                    ✕
                  </Button>
                </div>

                {(p.dataType === "select" || p.dataType === "multiselect") && (
                  <input
                    className={inputCls}
                    placeholder="Opções separadas por vírgula (ex.: Humano, Elfo, Androide)"
                    value={p.options}
                    onChange={(e) => patchProp(i, { options: e.target.value })}
                  />
                )}

                {p.dataType === "entity_ref" && (
                  <select
                    className={inputCls}
                    value={p.targetTypeId}
                    onChange={(e) => patchProp(i, { targetTypeId: e.target.value })}
                  >
                    <option value="">Qualquer tipo de entidade</option>
                    {types.map((t) => (
                      <option key={t.id} value={t.id}>
                        Somente: {t.name}
                      </option>
                    ))}
                  </select>
                )}

                <div className="flex items-center justify-between">
                  <label className="flex items-center gap-2 text-xs text-zinc-600 dark:text-zinc-400">
                    <input
                      type="checkbox"
                      checked={p.required}
                      onChange={(e) => patchProp(i, { required: e.target.checked })}
                    />
                    Obrigatória
                  </label>
                  {p.key && <span className="text-[10px] text-zinc-400">chave: {p.key}</span>}
                </div>
              </div>
            ))}
            <Button variant="secondary" onClick={() => setProps((prev) => [...prev, emptyProp()])}>
              + Adicionar propriedade
            </Button>
          </div>

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

        {/* Tipos existentes */}
        <section className="space-y-3">
          <h2 className="text-lg font-semibold">Tipos deste universo</h2>
          {types.length === 0 && (
            <p className="text-sm text-zinc-500">Nenhum tipo criado ainda.</p>
          )}
          {types.map((t) => (
            <div
              key={t.id}
              className="rounded-md border border-zinc-200 p-3 dark:border-zinc-800"
              style={{ borderLeft: `4px solid ${t.color ?? "#6366f1"}` }}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="font-medium">
                  {t.icon ? `${t.icon} ` : ""}
                  {t.name}
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
              {t.properties.length === 0 ? (
                <div className="text-xs text-zinc-500">Sem propriedades extras.</div>
              ) : (
                <ul className="mt-1 space-y-0.5 text-xs text-zinc-600 dark:text-zinc-400">
                  {t.properties.map((p) => (
                    <li key={p.id}>
                      {p.label}
                      {p.required && <span className="text-rose-500"> *</span>} —{" "}
                      {dataTypeLabel(p.dataType)}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))}
        </section>
      </div>

      <ConfirmDialog
        open={pendingRemoval !== null}
        danger
        title="Remover propriedades?"
        confirmLabel="Remover e salvar"
        message={
          <>
            <p>
              Propriedades removidas: <b>{pendingRemoval?.labels.join(", ")}</b>.
            </p>
            <p>
              Os valores delas serão apagados de todas as entidades deste tipo, e as relações
              geradas por elas também. Isso não pode ser desfeito.
            </p>
          </>
        }
        onConfirm={async () => {
          const p = pendingRemoval;
          setPendingRemoval(null);
          if (p) await save(p.edits);
        }}
        onCancel={() => setPendingRemoval(null)}
      />

      <ConfirmDialog
        open={deleting !== null}
        danger
        title={`Excluir o tipo "${deleting?.name ?? ""}"?`}
        confirmLabel="Excluir tipo"
        message={
          <p>
            Só é possível excluir tipos que não têm entidades. Se houver, o app avisará quantas
            são.
          </p>
        }
        onConfirm={removeType}
        onCancel={() => setDeleting(null)}
      />
    </div>
  );
}