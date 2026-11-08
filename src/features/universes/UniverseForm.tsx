import { useState, type FormEvent } from "react";
import { useApp } from "../../app/AppContext";
import { errMsg } from "../../shared/text";
import { Button, ErrorBanner, inputCls } from "../../shared/ui";

export function UniverseForm({ onDone }: { onDone?: () => void }) {
  const { createUniverse } = useApp();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await createUniverse(name, description.trim() || undefined);
      setName("");
      setDescription("");
      onDone?.();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="space-y-2">
      <ErrorBanner message={error} />
      <input
        className={inputCls}
        placeholder="Nome do universo"
        value={name}
        onChange={(e) => setName(e.target.value)}
        autoFocus
      />
      <input
        className={inputCls}
        placeholder="Descrição (opcional)"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
      />
      <div className="flex gap-2">
        <Button type="submit" disabled={busy || !name.trim()}>
          Criar universo
        </Button>
        {onDone && (
          <Button variant="ghost" onClick={onDone}>
            Cancelar
          </Button>
        )}
      </div>
    </form>
  );
}