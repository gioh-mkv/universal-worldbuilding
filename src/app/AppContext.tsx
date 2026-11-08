import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api, type EntityType, type ProjectInfo, type Universe } from "../shared/api";

interface AppCtx {
  project: ProjectInfo | null;
  universes: Universe[];
  universe: Universe | null;
  types: EntityType[];
  /** Muda quando os dados foram alterados fora da tela de entidades (tipos, universos). */
  dataVersion: number;
  invalidate: () => void;
  enterProject: (info: ProjectInfo) => Promise<void>;
  leaveProject: () => void;
  selectUniverse: (id: string | null) => Promise<void>;
  createUniverse: (name: string, description?: string) => Promise<void>;
  updateUniverse: (id: string, name: string, description?: string) => Promise<void>;
  deleteUniverse: (id: string) => Promise<void>;
  refreshTypes: () => Promise<void>;
}

const Ctx = createContext<AppCtx | null>(null);

export function useApp(): AppCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useApp fora do AppProvider");
  return c;
}

const byName = (a: Universe, b: Universe) => a.name.localeCompare(b.name);

export function AppProvider({ children }: { children: ReactNode }) {
  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [universes, setUniverses] = useState<Universe[]>([]);
  const [universeId, setUniverseId] = useState<string | null>(null);
  const [types, setTypes] = useState<EntityType[]>([]);
  const [dataVersion, setDataVersion] = useState(0);

  const invalidate = useCallback(() => setDataVersion((v) => v + 1), []);

  const loadTypes = useCallback(async (id: string | null) => {
    setTypes(id ? await api.listEntityTypes(id) : []);
  }, []);

  const selectUniverse = useCallback(
    async (id: string | null) => {
      setUniverseId(id);
      await loadTypes(id);
    },
    [loadTypes],
  );

  const enterProject = useCallback(
    async (info: ProjectInfo) => {
      const us = await api.listUniverses();
      setProject(info);
      setUniverses(us);
      await selectUniverse(us[0]?.id ?? null);
    },
    [selectUniverse],
  );

  const leaveProject = useCallback(() => {
    setProject(null);
    setUniverses([]);
    setUniverseId(null);
    setTypes([]);
  }, []);

  const createUniverse = useCallback(
    async (name: string, description?: string) => {
      const u = await api.createUniverse(name, description);
      setUniverses((prev) => [...prev, u].sort(byName));
      await selectUniverse(u.id);
    },
    [selectUniverse],
  );

  const updateUniverse = useCallback(
    async (id: string, name: string, description?: string) => {
      const u = await api.updateUniverse(id, name, description);
      setUniverses((prev) => prev.map((x) => (x.id === id ? u : x)).sort(byName));
    },
    [],
  );

  const deleteUniverse = useCallback(
    async (id: string) => {
      await api.deleteUniverse(id);
      const rest = universes.filter((u) => u.id !== id);
      setUniverses(rest);
      if (universeId === id) await selectUniverse(rest[0]?.id ?? null);
    },
    [universes, universeId, selectUniverse],
  );

  const refreshTypes = useCallback(() => loadTypes(universeId), [loadTypes, universeId]);

  const universe = universes.find((u) => u.id === universeId) ?? null;

  const value = useMemo<AppCtx>(
    () => ({
      project,
      universes,
      universe,
      types,
      dataVersion,
      invalidate,
      enterProject,
      leaveProject,
      selectUniverse,
      createUniverse,
      updateUniverse,
      deleteUniverse,
      refreshTypes,
    }),
    [
      project,
      universes,
      universe,
      types,
      dataVersion,
      invalidate,
      enterProject,
      leaveProject,
      selectUniverse,
      createUniverse,
      updateUniverse,
      deleteUniverse,
      refreshTypes,
    ],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}