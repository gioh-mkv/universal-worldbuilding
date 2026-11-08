import type { DataType } from "./api";

// "computed" fica de fora da interface até existir o motor de fórmulas.
export const DATA_TYPES: { value: DataType; label: string }[] = [
  { value: "text", label: "Texto" },
  { value: "number", label: "Número" },
  { value: "date", label: "Data" },
  { value: "datetime", label: "Data e hora" },
  { value: "boolean", label: "Sim / Não" },
  { value: "select", label: "Lista de opções" },
  { value: "multiselect", label: "Múltipla seleção" },
  { value: "entity_ref", label: "Referência a entidade" },
  { value: "coordinates", label: "Coordenadas" },
  { value: "image", label: "Imagem (caminho)" },
  { value: "file", label: "Arquivo (caminho)" },
];

export function dataTypeLabel(t: DataType): string {
  return DATA_TYPES.find((d) => d.value === t)?.label ?? t;
}