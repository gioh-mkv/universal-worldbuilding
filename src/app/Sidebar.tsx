import { useState } from "react";
import { UniverseForm } from "../features/universes/UniverseForm";
import { errMsg } from "../shared/text";
import type { Theme } from "../shared/theme";
import { Button, ErrorBanner, inputCls } from "../shared/ui";
import { useApp } from "./AppContext";

export type View =
  | "search"
  | "entities"
  | "types"
  | "relations"
  | "transfer"
  | "backup"
  | "settings";

const NAV: { id: View; icon: string; label: string; hint?: string }[] = [
  { id: "search", icon: "🔎", label: "Pesquisa", hint: "Ctrl+K" },
  { id: "entities", icon: "📚", label: "Entidades" },
  { id: "types", icon: "🧩", label: "Tipos de entidade" },
  { id: "relations", icon: "🔗", label: "Tipos de relação" },
  { id: "transfer", icon: "📦", label: "Exportar / Importar" },
  { id: "backup", icon: "💾", label: "Backup" },
  { id: "settings", icon: "⚙️", label: "Configurações" },
];

// Funcionam mesmo sem nenhum universo criado (importar e restaurar).
const WITHOUT_UNIVERSE: View[] = ["backup", "transfer"];

const NEXT_THEME: Record<Theme, Theme> = { light: "dark", dark: "system", system: "light" };
const THEME_ICON: Record<Theme, string> = { light: "☀️", dark: "🌙", system: "🖥️" };
const THEME_LABEL: Record<Theme, string> = {
  light: "Tema claro",
  dark: "Tema escuro",
  system: "Tema automático",
};

export function Sidebar({
  view,
  setView,
  theme,
  setTheme,
}: {
  view: View;
  setView: (v: View) => void;
  theme: Theme;
  setTheme: (t: Theme) => void;
}) {
  const { project, universes, universe, selectUniverse, leaveProject } = useApp();
  const [collapsed, setCollapsed] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  return (
    <aside
      className={`flex shrink-0 flex-col border-r border-zinc-200 bg-zinc-100 transition-all dark:border-zinc-800 dark:bg-zinc-900 ${
        collapsed ? "w-14" : "w-64"
      }`}
    >
      <div className="flex items-center justify-between p-2">
        {!collapsed && (
          <span className="px-2 text-sm font-semibold text-zinc-900 dark:text-zinc-100">UWB</span>
        )}
        <Button
          variant="ghost"
          onClick={() => setCollapsed((c) => !c)}
          title={collapsed ? "Expandir barra lateral" : "Recolher barra lateral"}
        >
          {collapsed ? "»" : "«"}
        </Button>
      </div>

      {!collapsed && (
        <div className="space-y-3 px-3 pb-3">
          <div>
            <div className="text-xs uppercase tracking-wide text-zinc-500">Projeto</div>
            <div className="flex items-center justify-between gap-2">
              <span className="truncate text-sm font-medium text-zinc-900 dark:text-zinc-100">
                {project?.name}
              </span>
              <Button variant="ghost" onClick={leaveProject}>
                Trocar
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <div className="text-xs uppercase tracking-wide text-zinc-500">Universo</div>
            <ErrorBanner message={error} onClose={() => setError(null)} />
            {creating ? (
              <UniverseForm onDone={() => setCreating(false)} />
            ) : (
              <div className="flex gap-2">
                <select
                  className={inputCls}
                  value={universe?.id ?? ""}
                  onChange={(e) =>
                    selectUniverse(e.target.value || null).catch((err) => setError(errMsg(err)))
                  }
                >
                  {universes.length === 0 && <option value="">Nenhum</option>}
                  {universes.map((u) => (
                    <option key={u.id} value={u.id}>
                      {u.name}
                    </option>
                  ))}
                </select>
                <Button variant="secondary" onClick={() => setCreating(true)} title="Novo universo">
                  +
                </Button>
              </div>
            )}
          </div>
        </div>
      )}

      <nav className="flex-1 space-y-1 px-2">
        {NAV.map((n) => (
          <button
            key={n.id}
            onClick={() => setView(n.id)}
            title={n.hint ? `${n.label} (${n.hint})` : n.label}
            disabled={!universe && !WITHOUT_UNIVERSE.includes(n.id)}
            className={`flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm disabled:opacity-40 ${
              view === n.id
                ? "bg-indigo-600 text-white"
                : "text-zinc-700 hover:bg-zinc-200 dark:text-zinc-300 dark:hover:bg-zinc-800"
            }`}
          >
            <span>{n.icon}</span>
            {!collapsed && <span>{n.label}</span>}
            {!collapsed && n.hint && <span className="ml-auto text-xs opacity-60">{n.hint}</span>}
          </button>
        ))}
      </nav>

      <div className="p-2">
        <Button
          variant="ghost"
          className="w-full justify-start"
          onClick={() => setTheme(NEXT_THEME[theme])}
          title={THEME_LABEL[theme]}
        >
          <span>{THEME_ICON[theme]}</span>
          {!collapsed && <span>{THEME_LABEL[theme]}</span>}
        </Button>
      </div>
    </aside>
  );
}