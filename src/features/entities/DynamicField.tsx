import type { Entity, PropertyDef } from "../../shared/api";
import { inputCls } from "../../shared/ui";

interface Props {
  def: PropertyDef;
  value: unknown;
  /** undefined remove a propriedade do registro */
  onChange: (v: unknown) => void;
  entities: Entity[];
  selfId?: string;
}

export function DynamicField({ def, value, onChange, entities, selfId }: Props) {
  const options = (def.config.options as string[] | undefined) ?? [];

  switch (def.dataType) {
    case "text":
    case "image":
    case "file":
      return (
        <input
          className={inputCls}
          value={(value as string | undefined) ?? ""}
          onChange={(e) => onChange(e.target.value || undefined)}
          placeholder={def.dataType === "text" ? "" : "Caminho (gestão de arquivos virá depois)"}
        />
      );

    case "number":
      return (
        <input
          type="number"
          className={inputCls}
          value={typeof value === "number" ? value : ""}
          onChange={(e) => onChange(e.target.value === "" ? undefined : Number(e.target.value))}
        />
      );

    case "date":
      return (
        <input
          type="date"
          className={inputCls}
          value={(value as string | undefined) ?? ""}
          onChange={(e) => onChange(e.target.value || undefined)}
        />
      );

    case "datetime":
      return (
        <input
          type="datetime-local"
          className={inputCls}
          value={(value as string | undefined) ?? ""}
          onChange={(e) => onChange(e.target.value || undefined)}
        />
      );

    case "boolean":
      return (
        <input
          type="checkbox"
          checked={value === true}
          onChange={(e) => onChange(e.target.checked)}
        />
      );

    case "select":
      return (
        <select
          className={inputCls}
          value={(value as string | undefined) ?? ""}
          onChange={(e) => onChange(e.target.value || undefined)}
        >
          <option value="">—</option>
          {options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      );

    case "multiselect": {
      const selected = Array.isArray(value) ? (value as string[]) : [];
      const toggle = (o: string) => {
        const next = selected.includes(o) ? selected.filter((s) => s !== o) : [...selected, o];
        onChange(next.length ? next : undefined);
      };
      return (
        <div className="flex flex-wrap gap-3">
          {options.map((o) => (
            <label key={o} className="flex items-center gap-1 text-sm">
              <input type="checkbox" checked={selected.includes(o)} onChange={() => toggle(o)} />
              {o}
            </label>
          ))}
        </div>
      );
    }

    case "entity_ref": {
      const target = def.config.targetTypeId as string | undefined;
      const candidates = entities.filter(
        (e) => e.id !== selfId && (!target || e.typeId === target),
      );
      return (
        <select
          className={inputCls}
          value={(value as string | undefined) ?? ""}
          onChange={(e) => onChange(e.target.value || undefined)}
        >
          <option value="">—</option>
          {candidates.map((e) => (
            <option key={e.id} value={e.id}>
              {e.name}
            </option>
          ))}
        </select>
      );
    }

    case "coordinates": {
      const c = (value && typeof value === "object" ? value : {}) as {
        lat?: number;
        lng?: number;
      };
      const set = (patch: { lat?: number; lng?: number }) => {
        const next = { ...c, ...patch };
        onChange(next.lat === undefined && next.lng === undefined ? undefined : next);
      };
      const parse = (s: string) => (s === "" ? undefined : Number(s));
      return (
        <div className="flex gap-2">
          <input
            type="number"
            step="any"
            className={inputCls}
            placeholder="Latitude / Y"
            value={c.lat ?? ""}
            onChange={(e) => set({ lat: parse(e.target.value) })}
          />
          <input
            type="number"
            step="any"
            className={inputCls}
            placeholder="Longitude / X"
            value={c.lng ?? ""}
            onChange={(e) => set({ lng: parse(e.target.value) })}
          />
        </div>
      );
    }

    case "computed":
      return <div className="text-sm text-zinc-500">Campo calculado (em breve).</div>;
  }
}