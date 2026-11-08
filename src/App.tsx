import { useCallback, useEffect, useState } from "react";
import { AppProvider, useApp } from "./app/AppContext";
import { Sidebar, type View } from "./app/Sidebar";
import { BackupView } from "./features/backup/BackupView";
import { EntitiesView } from "./features/entities/EntitiesView";
import { EntityTypeManager } from "./features/entity-types/EntityTypeManager";
import { ProjectPicker } from "./features/projects/ProjectPicker";
import { RelationTypeManager } from "./features/relation-types/RelationTypeManager";
import { SearchView } from "./features/search/SearchView";
import { ExportView } from "./features/transfer/ExportView";
import { UniverseForm } from "./features/universes/UniverseForm";
import { UniverseSettings } from "./features/universes/UniverseSettings";
import { api } from "./shared/api";
import { useTheme } from "./shared/theme";

const AUTO_BACKUP_CHECK_MS = 15 * 60 * 1000;

function Shell() {
  const { project, universe } = useApp();
  const { theme, setTheme } = useTheme();
  const [view, setView] = useState<View>("entities");
  const [openId, setOpenId] = useState<string | null>(null);

  const clearOpenId = useCallback(() => setOpenId(null), []);

  // Ctrl+K abre a pesquisa de qualquer tela.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setView("search");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Backup automático: ao abrir o projeto e depois a cada 15 minutos
  // (o backend só cria um se o intervalo configurado já passou).
  const projectPath = project?.path;
  useEffect(() => {
    if (!projectPath) return;
    const run = () =>
      api.runAutoBackup().catch((e) => console.error("backup automático falhou:", e));
    run();
    const timer = setInterval(run, AUTO_BACKUP_CHECK_MS);
    return () => clearInterval(timer);
  }, [projectPath]);

  if (!project) return <ProjectPicker />;

  return (
    <div className="flex h-full bg-white text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <Sidebar view={view} setView={setView} theme={theme} setTheme={setTheme} />
      <main className="min-w-0 flex-1 overflow-hidden">
        {view === "backup" ? (
          <BackupView />
        ) : view === "transfer" ? (
          <ExportView />
        ) : !universe ? (
          <div className="mx-auto max-w-md space-y-4 p-8">
            <h2 className="text-xl font-semibold">Crie o primeiro universo</h2>
            <p className="text-sm text-zinc-500">
              Um projeto pode conter vários universos. Comece criando um, importe um arquivo na
              aba Exportar / Importar, ou restaure um backup na aba Backup.
            </p>
            <UniverseForm />
          </div>
        ) : (
          <>
            {/* Sempre montada (só oculta) para preservar edições não salvas. */}
            <div className={view === "entities" ? "h-full" : "hidden"}>
              <EntitiesView
                onGoToTypes={() => setView("types")}
                openId={openId}
                onOpenHandled={clearOpenId}
              />
            </div>
            {view === "types" && <EntityTypeManager />}
            {view === "relations" && <RelationTypeManager />}
            {view === "settings" && <UniverseSettings />}
            {view === "search" && (
              <SearchView
                onOpen={(id) => {
                  setOpenId(id);
                  setView("entities");
                }}
              />
            )}
          </>
        )}
      </main>
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <Shell />
    </AppProvider>
  );
}