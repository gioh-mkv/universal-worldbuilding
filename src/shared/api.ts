import { invoke } from "@tauri-apps/api/core";

export type DataType =
  | "text" | "number" | "date" | "datetime" | "boolean" | "select"
  | "multiselect" | "image" | "file" | "entity_ref" | "coordinates" | "computed";

export type RichDoc = Record<string, unknown>;
export type Directionality = "directed" | "bidirectional" | "hierarchical";

export interface ProjectInfo { name: string; path: string }
export interface Universe { id: string; name: string; description: string | null }

export interface PropertyDef {
  id: string; key: string; label: string; dataType: DataType;
  config: Record<string, unknown>; required: boolean; position: number;
}
export interface NewPropertyDef {
  key: string; label: string; dataType: DataType;
  config?: Record<string, unknown>; required?: boolean;
}
export interface EntityType {
  id: string; universeId: string; name: string;
  icon: string | null; color: string | null; properties: PropertyDef[];
}
export interface NewEntityType {
  universeId: string; name: string; icon?: string; color?: string;
  properties?: NewPropertyDef[];
}
/** Existente: informe `id`. Nova: informe `key` e `dataType`. */
export interface PropertyEdit {
  id?: string; key?: string; dataType?: DataType;
  label: string; config?: Record<string, unknown>; required?: boolean;
}
/** Propriedades existentes que não estiverem na lista são removidas. */
export interface UpdateEntityType {
  id: string; name: string; icon?: string; color?: string;
  properties: PropertyEdit[];
}

export interface Entity {
  id: string; universeId: string; typeId: string; name: string;
  summary: string | null; properties: Record<string, unknown>;
  bodyJson: RichDoc | null; bodyMd: string | null;
  createdAt: string; updatedAt: string;
}
export interface NewEntity {
  universeId: string; typeId: string; name: string; summary?: string;
  properties?: Record<string, unknown>; bodyJson?: RichDoc;
}
export interface UpdateEntity {
  id: string; name?: string; summary?: string;
  properties?: Record<string, unknown>; bodyJson?: RichDoc;
}

export interface EntityRef { id: string; name: string; typeId: string }
export interface EntityLinks { outgoing: EntityRef[]; incoming: EntityRef[] }

export interface RelationType {
  id: string; universeId: string; name: string;
  inverseName: string | null; directionality: Directionality;
  usageCount: number;
}
export interface NewRelationType {
  universeId: string; name: string; inverseName?: string; directionality?: Directionality;
}
export interface UpdateRelationType {
  id: string; name: string; inverseName?: string; directionality: Directionality;
}
export interface NewRelation {
  universeId: string; typeId: string; sourceId: string; targetId: string;
  description?: string; startDate?: string; endDate?: string;
}
export interface RelationView {
  id: string; typeId: string; label: string;
  direction: "outgoing" | "incoming" | "both";
  directionality: Directionality; other: EntityRef;
  description: string | null; startDate: string | null; endDate: string | null;
  propertyKey: string | null;
}

export interface RelationFilter { entityId: string; relationTypeId?: string }
export interface PropertyFilter { key: string; value: string }
export interface SearchQuery {
  universeId: string;
  text?: string;
  typeIds?: string[];
  relatedTo?: RelationFilter[];
  mentioning?: string[];
  propertyFilters?: PropertyFilter[];
  limit?: number;
}
export interface SearchHit {
  id: string; name: string; typeId: string; summary: string | null;
  /** Trechos marcados com \uE000 (início) e \uE001 (fim). */
  nameHl: string | null; snippet: string | null;
}

export type BackupKind = "manual" | "auto" | "pre-restore";
export interface BackupSettings { autoEnabled: boolean; intervalHours: number; keepAuto: number }
export interface BackupCounts {
  universes: number; entities: number; entityTypes: number; relations: number;
}
export interface BackupInfo {
  id: string; formatVersion: number; kind: BackupKind; label: string | null;
  createdAt: string; projectName: string; schemaVersion: number; appVersion: string;
  sizeBytes: number; sha256: string; counts: BackupCounts; includesAssets: boolean;
}
export interface IntegrityCheck { name: string; ok: boolean; detail: string }
export interface IntegrityReport { ok: boolean; checks: IntegrityCheck[] }

export interface ExportResult { path: string; entities: number }
export interface ImportSummary {
  universeId: string; universeName: string;
  entityTypes: number; relationTypes: number; entities: number; relations: number;
  warnings: string[];
}

export const api = {
  listProjects: () => invoke<ProjectInfo[]>("list_projects"),
  createProject: (name: string) => invoke<ProjectInfo>("create_project", { name }),
  openProject: (path: string) => invoke<ProjectInfo>("open_project", { path }),
  duplicateProject: (path: string, newName: string) =>
    invoke<ProjectInfo>("duplicate_project", { path, newName }),
  deleteProject: (path: string) => invoke<void>("delete_project", { path }),

  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  createBackup: (label?: string) => invoke<BackupInfo>("create_backup", { label }),
  verifyBackup: (id: string) => invoke<IntegrityReport>("verify_backup", { id }),
  deleteBackup: (id: string) => invoke<void>("delete_backup", { id }),
  restoreBackup: (id: string) => invoke<ProjectInfo>("restore_backup", { id }),
  runAutoBackup: () => invoke<BackupInfo | null>("run_auto_backup"),
  checkProjectIntegrity: () => invoke<IntegrityReport>("check_project_integrity"),
  getBackupSettings: () => invoke<BackupSettings>("get_backup_settings"),
  setBackupSettings: (settings: BackupSettings) =>
    invoke<BackupSettings>("set_backup_settings", { settings }),

  exportUniverseJson: (universeId: string, path: string) =>
    invoke<ExportResult>("export_universe_json", { universeId, path }),
  exportUniverseMarkdown: (universeId: string, dir: string) =>
    invoke<ExportResult>("export_universe_markdown", { universeId, dir }),
  importUniverseJson: (path: string, newName?: string) =>
    invoke<ImportSummary>("import_universe_json", { path, newName }),

  createUniverse: (name: string, description?: string) =>
    invoke<Universe>("create_universe", { name, description }),
  listUniverses: () => invoke<Universe[]>("list_universes"),
  updateUniverse: (id: string, name: string, description?: string) =>
    invoke<Universe>("update_universe", { id, name, description }),
  deleteUniverse: (id: string) => invoke<void>("delete_universe", { id }),

  createEntityType: (input: NewEntityType) =>
    invoke<EntityType>("create_entity_type", { input }),
  updateEntityType: (input: UpdateEntityType) =>
    invoke<EntityType>("update_entity_type", { input }),
  deleteEntityType: (id: string) => invoke<void>("delete_entity_type", { id }),
  listEntityTypes: (universeId: string) =>
    invoke<EntityType[]>("list_entity_types", { universeId }),

  createEntity: (input: NewEntity) => invoke<Entity>("create_entity", { input }),
  updateEntity: (input: UpdateEntity) => invoke<Entity>("update_entity", { input }),
  deleteEntity: (id: string) => invoke<void>("delete_entity", { id }),
  listEntities: (universeId: string, typeId?: string) =>
    invoke<Entity[]>("list_entities", { universeId, typeId }),

  getEntityLinks: (entityId: string) =>
    invoke<EntityLinks>("get_entity_links", { entityId }),

  createRelationType: (input: NewRelationType) =>
    invoke<RelationType>("create_relation_type", { input }),
  updateRelationType: (input: UpdateRelationType) =>
    invoke<RelationType>("update_relation_type", { input }),
  deleteRelationType: (id: string) => invoke<void>("delete_relation_type", { id }),
  listRelationTypes: (universeId: string) =>
    invoke<RelationType[]>("list_relation_types", { universeId }),
  createRelation: (input: NewRelation) => invoke<string>("create_relation", { input }),
  deleteRelation: (id: string) => invoke<void>("delete_relation", { id }),
  listEntityRelations: (entityId: string) =>
    invoke<RelationView[]>("list_entity_relations", { entityId }),

  search: (query: SearchQuery) => invoke<SearchHit[]>("search", { query }),
};