import { useMemo, useState, type FormEvent } from "react";
import type { JSONContent } from "@tiptap/react";
import { useApp } from "../../app/AppContext";
import { api, type Entity } from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ConfirmDialog, ErrorBanner, Field, inputCls } from "../../shared/ui";
import { DynamicField } from "./DynamicField";
import { LinksPanel } from "./LinksPanel";
import { RelationsPanel } from "./RelationsPanel";
import { RichEditor } from "./RichEditor";
import { initialDoc, refreshMentionLabels } from "./richDoc";

interface Props {
  entity: Entity | null; // null = criando
  defaultTypeId: string | null;
  entities: Entity[]; // referências, menções e relações
  onSaved: (e: Entity) => void;
  onCancel: () => void;
  onNavigate: (id: string) => void;
  onDeleted: () => void;
}

export function EntityForm({
  entity,
  defaultTypeId,
  entities,
  onSaved,
  onCancel,
  onNavigate,
  onDeleted,
}: Props) {
  const { universe, types } = useApp();
  const [typeId, setTypeId] = useState(entity?.typeId ?? defaultTypeId ?? types[0]?.id ?? "");
  const [name, setName] = useState(entity?.name ?? "");
  const [summary, setSummary] = useState(entity?.summary ?? "");
  const [values, setValues] = useState<Record<string, unknown>>(entity?.properties ?? {});
  // undefined = documento não alterado (não é reenviado ao salvar)
  const [doc, setDoc] = useState<JSONContent | undefined>(undefined);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const type = types.find((t) => t.id === typeId);

  // O editor só lê o conteúdo inicial ao ser criado, então calculamos uma única vez.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const startDoc = useMemo(
    () =>
      refreshMentionLabels(
        initialDoc(entity),
        new Map(entities.map((e) => [e.id, e.name])),
      ),
    [],
  );

  function setValue(key: string, v: unknown) {
    setDirty(true);
    setValues((prev) => {
      const next = { ...prev };
      if (v === undefined) delete next[key];
      else next[key] = v;
      return next;
    });
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!universe || !type) return;
    setError(null);
    setBusy(true);
    try {
      // Garante que só vão propriedades definidas no tipo.
      const properties = Object.fromEntries(
        Object.entries(values).filter(([k]) => type.properties.some((p) => p.key === k)),
      );
      const saved = entity
        ? await api.updateEntity({ id: entity.id, name, summary, properties, bodyJson: doc })
        : await api.createEntity({
            universeId: universe.id,
            typeId: type.id,
            name,
            summary: summary || undefined,
            properties,
            bodyJson: doc,
          });
      setDirty(false);
      setDoc(undefined);
      onSaved(saved);
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (!entity) return;
    try {
      await api.deleteEntity(entity.id);
      setConfirmDelete(false);
      onDeleted();
    } catch (err) {
      setConfirmDelete(false);
      setError(errMsg(err));
    }
  }

  return (
    <form onSubmit={submit} className="mx-auto max-w-3xl space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">
          {entity ? `Editar: ${entity.name}` : "Nova entidade"}
        </h2>
        <div className="text-xs text-zinc-500">
          {dirty && <span className="mr-3 text-amber-600">● Alterações não salvas</span>}
          {entity && <span>Atualizado em {entity.updatedAt} (UTC)</span>}
        </div>
      </div>

      <ErrorBanner message={error} onClose={() => setError(null)} />

      <Field label="Tipo" required>
        <select
          className={inputCls}
          value={typeId}
          disabled={!!entity}
          onChange={(e) => {
            setTypeId(e.target.value);
            setValues({});
            setDirty(true);
          }}
        >
          {types.map((t) => (
            <option key={t.id} value={t.id}>
              {t.icon ? `${t.icon} ` : ""}
              {t.name}
            </option>
          ))}
        </select>
      </Field>

      <Field label="Nome" required>
        <input
          className={inputCls}
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setDirty(true);
          }}
        />
      </Field>

      <Field label="Resumo">
        <input
          className={inputCls}
          value={summary}
          onChange={(e) => {
            setSummary(e.target.value);
            setDirty(true);
          }}
        />
      </Field>

      {type?.properties.map((def) => (
        <Field key={def.id} label={def.label} required={def.required}>
          <DynamicField
            def={def}
            value={values[def.key]}
            onChange={(v) => setValue(def.key, v)}
            entities={entities}
            selfId={entity?.id}
          />
        </Field>
      ))}

      <Field
        label="Documento"
        hint="Digite [[ para mencionar outra entidade. Ctrl+clique em uma menção abre o registro."
      >
        <RichEditor
          initial={startDoc}
          entities={entities}
          selfId={entity?.id}
          onChange={(d) => {
            setDoc(d);
            setDirty(true);
          }}
          onNavigate={onNavigate}
        />
      </Field>

      <div className="flex items-center gap-2">
        <Button type="submit" disabled={busy || !name.trim() || !type}>
          {entity ? "Salvar alterações" : "Criar entidade"}
        </Button>
        <Button variant="ghost" onClick={onCancel}>
          Cancelar
        </Button>
        {entity && (
          <Button variant="danger" className="ml-auto" onClick={() => setConfirmDelete(true)}>
            Excluir
          </Button>
        )}
      </div>

      {entity && (
        <div className="space-y-6 border-t border-zinc-200 pt-4 dark:border-zinc-800">
          <RelationsPanel entity={entity} entities={entities} onNavigate={onNavigate} />
          <LinksPanel entity={entity} onNavigate={onNavigate} />
        </div>
      )}

      <ConfirmDialog
        open={confirmDelete}
        danger
        title={`Excluir "${entity?.name ?? ""}"?`}
        confirmLabel="Excluir"
        message={
          <>
            <p>
              Menções a esta entidade em outros documentos viram texto simples, e as relações e o
              histórico dela são apagados.
            </p>
            <p className="font-medium">Essa ação não pode ser desfeita.</p>
          </>
        }
        onConfirm={remove}
        onCancel={() => setConfirmDelete(false)}
      />
    </form>
  );
}