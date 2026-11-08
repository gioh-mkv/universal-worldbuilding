import { useEffect, useState } from "react";
import { useApp } from "../../app/AppContext";
import { api, type Entity, type EntityLinks, type EntityRef } from "../../shared/api";
import { errMsg } from "../../shared/text";

export function LinksPanel({
  entity,
  onNavigate,
}: {
  entity: Entity;
  onNavigate: (id: string) => void;
}) {
  const { types } = useApp();
  const [links, setLinks] = useState<EntityLinks | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Recarrega a cada salvamento (updatedAt muda).
  useEffect(() => {
    let alive = true;
    api
      .getEntityLinks(entity.id)
      .then((l) => alive && setLinks(l))
      .catch((e) => alive && setError(errMsg(e)));
    return () => {
      alive = false;
    };
  }, [entity.id, entity.updatedAt]);

  if (error) return <p className="text-xs text-rose-500">{error}</p>;
  if (!links) return null;

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Group title="Mencionada em" refs={links.incoming} onNavigate={onNavigate} types={types} />
      <Group title="Menciona" refs={links.outgoing} onNavigate={onNavigate} types={types} />
    </div>
  );
}

function Group({
  title,
  refs,
  onNavigate,
  types,
}: {
  title: string;
  refs: EntityRef[];
  onNavigate: (id: string) => void;
  types: { id: string; color: string | null }[];
}) {
  return (
    <section>
      <h3 className="mb-1 text-xs font-medium uppercase tracking-wide text-zinc-500">{title}</h3>
      {refs.length === 0 ? (
        <p className="text-xs text-zinc-500">Nenhuma.</p>
      ) : (
        <ul className="space-y-1">
          {refs.map((r) => (
            <li key={r.id}>
              <button
                type="button"
                onClick={() => onNavigate(r.id)}
                className="flex items-center gap-2 text-sm text-indigo-600 hover:underline dark:text-indigo-400"
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ background: types.find((t) => t.id === r.typeId)?.color ?? "#6366f1" }}
                />
                {r.name}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}