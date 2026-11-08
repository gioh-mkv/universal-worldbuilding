import { open, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { useApp } from "../../app/AppContext";
import { api, type ImportSummary } from "../../shared/api";
import { errMsg, slugKey } from "../../shared/text";
import { Button, ErrorBanner } from "../../shared/ui";

export function ExportView() {
  const { project, universe, enterProject, selectUniverse, invalidate } = useApp();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [summary, setSummary] = useState<ImportSummary | null>(null);

  async function run(key: string, fn: () => Promise<void>) {
    setBusy(key);
    setError(null);
    setNotice(null);
    setSummary(null);
    try {
      await fn();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(null);
    }
  }

  const exportJson = () =>
    run("json", async () => {
      if (!universe) return;
      const base = slugKey(universe.name).replace(/_/g, "-") || "universo";
      const chosen = await save({
        title: "Exportar universo (JSON)",
        defaultPath: `${base}.uwb.json`,
        filters: [{ name: "Universo UWB", extensions: ["json"] }],
      });
      if (!chosen) return;
      const path = chosen.toLowerCase().endsWith(".json") ? chosen : `${chosen}.json`;
      const r = await api.exportUniverseJson(universe.id, path);
      setNotice(`Exportado: ${r.entities} entidade(s) em ${r.path}`);
    });

  const exportMarkdown = () =>
    run("md", async () => {
      if (!universe) return;
      const dir = await open({
        title: "Escolha a pasta onde criar a exportação",
        directory: true,
        multiple: false,
      });
      if (typeof dir !== "string") return;
      const r = await api.exportUniverseMarkdown(universe.id, dir);
      setNotice(`Exportado: ${r.entities} nota(s) em ${r.path}`);
    });

  const importJson = () =>
    run("import", async () => {
      const file = await open({
        title: "Escolha o arquivo de universo (.json)",
        directory: false,
        multiple: false,
        filters: [{ name: "Universo UWB", extensions: ["json"] }],
      });
      if (typeof file !== "string") return;
      const s = await api.importUniverseJson(file);
      if (project) {
        await enterProject(project); // recarrega a lista de universos
        await selectUniverse(s.universeId); // e abre o universo importado
      }
      invalidate();
      setSummary(s);
    });

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto max-w-3xl space-y-8">
        <header>
          <h2 className="text-lg font-semibold">Exportar e importar</h2>
          <p className="text-sm text-zinc-500">
            Seus dados são seus: leve um universo para outro projeto, outro computador ou para o
            Obsidian.
          </p>
        </header>

        <ErrorBanner message={error} onClose={() => setError(null)} />
        {notice && (
          <div className="rounded-md border border-emerald-300 bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-200">
            {notice}
          </div>
        )}

        <section className="space-y-4 rounded-md border border-zinc-200 p-4 dark:border-zinc-800">
          <h3 className="text-sm font-semibold">
            Exportar {universe ? `“${universe.name}”` : "universo"}
          </h3>

          <div className="space-y-1">
            <Button onClick={exportJson} disabled={!universe || busy !== null}>
              {busy === "json" ? "Exportando…" : "Exportar JSON completo"}
            </Button>
            <p className="text-xs text-zinc-500">
              Um único arquivo com tipos, propriedades, entidades, documentos e relações. É o
              formato para importar de volta no UWB.
            </p>
          </div>

          <div className="space-y-1">
            <Button variant="secondary" onClick={exportMarkdown} disabled={!universe || busy !== null}>
              {busy === "md" ? "Exportando…" : "Exportar Markdown (Obsidian)"}
            </Button>
            <p className="text-xs text-zinc-500">
              Uma pasta com uma nota por entidade, propriedades no cabeçalho e links{" "}
              <code>[[Nome]]</code> preservados. Não pode ser importada de volta.
            </p>
          </div>
        </section>

        <section className="space-y-3 rounded-md border border-zinc-200 p-4 dark:border-zinc-800">
          <h3 className="text-sm font-semibold">Importar universo</h3>
          <p className="text-xs text-zinc-500">
            A importação sempre cria um <b>novo universo</b> neste projeto; nada do que já existe é
            alterado. Se houver um universo com o mesmo nome, o novo recebe “(importado)”.
          </p>
          <Button variant="secondary" onClick={importJson} disabled={busy !== null}>
            {busy === "import" ? "Importando…" : "Escolher arquivo .json…"}
          </Button>

          {summary && (
            <div className="space-y-2 rounded-md bg-zinc-100 p-3 text-sm dark:bg-zinc-900">
              <div className="font-medium text-emerald-700 dark:text-emerald-400">
                Universo “{summary.universeName}” importado.
              </div>
              <div className="text-xs text-zinc-600 dark:text-zinc-400">
                {summary.entities} entidades · {summary.entityTypes} tipos ·{" "}
                {summary.relations} relações · {summary.relationTypes} tipos de relação
              </div>
              {summary.warnings.length > 0 && (
                <div>
                  <div className="text-xs font-medium text-amber-700 dark:text-amber-400">
                    Avisos ({summary.warnings.length}):
                  </div>
                  <ul className="ml-4 list-disc text-xs text-zinc-600 dark:text-zinc-400">
                    {summary.warnings.map((w, i) => (
                      <li key={i}>{w}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}