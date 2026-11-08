import { useCallback, useEffect, useState, type FormEvent } from "react";
import { useApp } from "../../app/AppContext";
import { api, type ProjectInfo } from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ConfirmDialog, ErrorBanner, Field, inputCls } from "../../shared/ui";

export function ProjectPicker() {
  const { enterProject } = useApp();
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [duplicating, setDuplicating] = useState<ProjectInfo | null>(null);
  const [copyName, setCopyName] = useState("");
  const [deleting, setDeleting] = useState<ProjectInfo | null>(null);

  const reload = useCallback(async () => {
    try {
      setProjects(await api.listProjects());
    } catch (e) {
      setError(errMsg(e));
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  async function open(p: ProjectInfo) {
    setBusy(true);
    setError(null);
    try {
      await enterProject(await api.openProject(p.path));
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  async function create(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await enterProject(await api.createProject(name));
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  function startDuplicate(p: ProjectInfo) {
    setDuplicating(p);
    setCopyName(`${p.name} (cópia)`);
    setError(null);
  }

  async function duplicate(e: FormEvent) {
    e.preventDefault();
    if (!duplicating || !copyName.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await api.duplicateProject(duplicating.path, copyName);
      setDuplicating(null);
      await reload();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (!deleting) return;
    const target = deleting;
    setDeleting(null);
    setBusy(true);
    setError(null);
    try {
      await api.deleteProject(target.path);
      await reload();
    } catch (err) {
      setError(errMsg(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex h-full items-center justify-center overflow-y-auto bg-zinc-50 p-6 dark:bg-zinc-950">
      <div className="w-full max-w-lg space-y-6">
        <header>
          <h1 className="text-2xl font-semibold text-zinc-900 dark:text-zinc-100">
            Universal Worldbuilding
          </h1>
          <p className="text-sm text-zinc-500">
            Abra um projeto existente ou crie um novo.
          </p>
        </header>

        <ErrorBanner message={error} onClose={() => setError(null)} />

        <form
          onSubmit={create}
          className="space-y-3 rounded-lg border border-zinc-200 bg-white p-4 dark:border-zinc-800 dark:bg-zinc-900"
        >
          <Field label="Novo projeto">
            <input
              className={inputCls}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Ex.: Crônicas de Aurora"
            />
          </Field>
          <Button type="submit" disabled={busy || !name.trim()}>
            Criar projeto
          </Button>
        </form>

        <section className="space-y-2">
          <h2 className="text-sm font-medium text-zinc-600 dark:text-zinc-400">
            Projetos existentes
          </h2>
          {projects.length === 0 && (
            <p className="text-sm text-zinc-500">Nenhum projeto ainda.</p>
          )}
          {projects.map((p) => (
            <div
              key={p.path}
              className="rounded-lg border border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-900"
            >
              <div className="flex items-center gap-1 pr-2">
                <button
                  disabled={busy}
                  onClick={() => open(p)}
                  className="min-w-0 flex-1 rounded-lg px-4 py-3 text-left hover:bg-zinc-50 disabled:opacity-50 dark:hover:bg-zinc-800"
                >
                  <div className="font-medium text-zinc-900 dark:text-zinc-100">{p.name}</div>
                  <div className="truncate text-xs text-zinc-500">{p.path}</div>
                </button>
                <Button variant="ghost" disabled={busy} onClick={() => startDuplicate(p)}>
                  Duplicar
                </Button>
                <Button variant="ghost" disabled={busy} onClick={() => setDeleting(p)}>
                  Excluir
                </Button>
              </div>

              {duplicating?.path === p.path && (
                <form
                  onSubmit={duplicate}
                  className="flex gap-2 border-t border-zinc-200 p-3 dark:border-zinc-800"
                >
                  <input
                    className={inputCls}
                    value={copyName}
                    onChange={(e) => setCopyName(e.target.value)}
                    autoFocus
                  />
                  <Button type="submit" disabled={busy || !copyName.trim()}>
                    Duplicar
                  </Button>
                  <Button variant="ghost" onClick={() => setDuplicating(null)}>
                    Cancelar
                  </Button>
                </form>
              )}
            </div>
          ))}
        </section>
      </div>

      <ConfirmDialog
        open={deleting !== null}
        danger
        title={`Excluir o projeto "${deleting?.name ?? ""}"?`}
        confirmLabel="Excluir projeto"
        requireText={deleting?.name}
        message={
          <>
            <p>
              A pasta inteira do projeto será apagada do disco: banco de dados, arquivos e{" "}
              <b>também os backups que estão dentro dela</b>.
            </p>
            <p className="font-medium">Essa ação não pode ser desfeita.</p>
          </>
        }
        onConfirm={remove}
        onCancel={() => setDeleting(null)}
      />
    </div>
  );
}