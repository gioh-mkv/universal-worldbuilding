import { useEffect, useState, type FormEvent } from "react";
import { useApp } from "../../app/AppContext";
import { errMsg } from "../../shared/text";
import { Button, ConfirmDialog, ErrorBanner, Field, inputCls } from "../../shared/ui";

export function UniverseSettings() {
  const { universe, updateUniverse, deleteUniverse } = useApp();
  const [name, setName] = useState(universe?.name ?? "");
  const [description, setDescription] = useState(universe?.description ?? "");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  // Reinicia o formulário ao trocar de universo.
  useEffect(() => {
    setName(universe?.name ?? "");
    setDescription(universe?.description ?? "");
    setSaved(false);
    setError(null);
  }, [universe?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!universe) return null;

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!universe) return;
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      await updateUniverse(universe.id, name, description.trim() || undefined);
      setSaved(true);
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto max-w-xl space-y-8">
        <form onSubmit={submit} className="space-y-4">
          <h2 className="text-lg font-semibold">Configurações do universo</h2>
          <ErrorBanner message={error} onClose={() => setError(null)} />

          <Field label="Nome" required>
            <input
              className={inputCls}
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setSaved(false);
              }}
            />
          </Field>

          <Field label="Descrição">
            <textarea
              className={inputCls}
              rows={4}
              value={description}
              onChange={(e) => {
                setDescription(e.target.value);
                setSaved(false);
              }}
            />
          </Field>

          <div className="flex items-center gap-3">
            <Button type="submit" disabled={busy || !name.trim()}>
              Salvar
            </Button>
            {saved && <span className="text-sm text-emerald-600">Salvo.</span>}
          </div>
        </form>

        <section className="space-y-3 rounded-md border border-rose-300 p-4 dark:border-rose-900">
          <h3 className="font-semibold text-rose-700 dark:text-rose-300">Zona de perigo</h3>
          <p className="text-sm text-zinc-600 dark:text-zinc-400">
            Excluir o universo apaga todos os tipos, entidades, documentos e relações que estão
            dentro dele.
          </p>
          <Button variant="danger" onClick={() => setConfirming(true)}>
            Excluir este universo
          </Button>
        </section>
      </div>

      <ConfirmDialog
        open={confirming}
        danger
        title={`Excluir "${universe.name}"?`}
        confirmLabel="Excluir universo"
        requireText={universe.name}
        message={<p>Essa ação não pode ser desfeita.</p>}
        onConfirm={async () => {
          try {
            await deleteUniverse(universe.id);
          } catch (err) {
            setError(errMsg(err));
          }
          setConfirming(false);
        }}
        onCancel={() => setConfirming(false)}
      />
    </div>
  );
}