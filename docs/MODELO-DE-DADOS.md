# Modelo de dados

## Tabelas

| Tabela | Colunas principais | Observações |
|---|---|---|
| `universes` | `id`, `name`, `description`, `settings`, `created_at`, `updated_at` | `settings` é reservado. |
| `entity_types` | `id`, `universe_id`, `name`, `icon`, `color`, `position` | Nome único por universo. |
| `property_defs` | `id`, `type_id`, `key`, `label`, `data_type`, `config`, `required`, `position` | `key` única por tipo. |
| `entities` | `id`, `universe_id`, `type_id`, `name`, `summary`, `properties`, `body_json`, `body_md`, `created_at`, `updated_at` | `properties` e `body_json` são JSON em texto. |
| `relation_types` | `id`, `universe_id`, `name`, `inverse_name`, `directionality` | `directed`, `bidirectional` ou `hierarchical`. |
| `relations` | `id`, `universe_id`, `type_id`, `source_id`, `target_id`, `description`, `start_date`, `end_date`, `property_key` | `property_key` preenchido = nasceu de uma propriedade `entity_ref`. |
| `mentions` | `source_id`, `target_id` | Derivada dos documentos. |
| `entity_versions` | `id`, `entity_id`, `snapshot`, `created_at` | Snapshot JSON a cada edição. A interface de histórico ainda não existe. |
| `search_index` | tabela virtual FTS5 | Colunas `entity_id` e `universe_id` (não indexadas), `name`, `summary`, `body`, `props`. |

Colunas `properties` em `relations` e `description` em `entity_types` existem, mas ainda não são usadas.

## Tipos de dado e valores

| `data_type` | Valor em `entities.properties` | `config` |
|---|---|---|
| `text`, `date`, `datetime`, `image`, `file` | texto | |
| `number` | número | |
| `boolean` | `true` / `false` | |
| `select` | texto, uma das opções | `{ "options": ["A", "B"] }` |
| `multiselect` | lista de textos, todos entre as opções | `{ "options": [...] }` |
| `entity_ref` | id da entidade alvo | `{ "targetTypeId": "…" }` (opcional) |
| `coordinates` | `{ "lat": n, "lng": n }` | |
| `computed` | reservado (sem motor de fórmulas ainda) | |

## Comportamento ao excluir

| Operação | Efeito no banco |
|---|---|
| Excluir entidade | Menções viram texto nos outros documentos; propriedades `entity_ref` que a citavam são removidas; `relations`, `mentions` e `entity_versions` dela somem; sai do `search_index`. |
| Excluir universo | Remove relações, índice e entidades (em ordem explícita), depois o universo (cascata para tipos, propriedades e tipos de relação). |
| Remover propriedade | `json_remove` em todas as entidades do tipo, remove as relações geradas por ela e reindexa. |

## Migrações

- Ficam em `src-tauri/migrations/NNN_nome.sql` e são listadas em `MIGRATIONS` (`src-tauri/src/db/mod.rs`).
- A versão aplicada é `PRAGMA user_version`. Ao abrir um projeto, as migrações pendentes rodam em transações.
- **Regra de ouro: nunca edite uma migração já publicada.** Corrija com uma migração nova.
- Não há reversão. Backups de esquema mais novo são recusados pelo app mais antigo.

## Estrutura de um projeto no disco

```
<projeto>/
├─ project.json     { "name": "...", "backup": { "autoEnabled": true, "intervalHours": 24, "keepAuto": 10 } }
├─ project.db       SQLite (modo WAL: pode existir project.db-wal e project.db-shm)
├─ assets/          reservado
└─ backups/
   └─ 20261107-193000-manual/
      ├─ project.db
      └─ manifest.json
```

**`manifest.json` de um backup:** `formatVersion`, `kind` (`manual`, `auto` ou `pre-restore`), `label`, `createdAt` (UTC), `projectName`, `schemaVersion`, `appVersion`, `sizeBytes`, `sha256`, `counts` e `includesAssets`.

## Formato de exportação (JSON)

Arquivo `formato = "uwb-universe"`, `formatVersion = 1`:

```json
{
  "format": "uwb-universe",
  "formatVersion": 1,
  "exportedAt": "2026-11-07T19:30:00Z",
  "appVersion": "0.1.0",
  "universe": { "id": "…", "name": "…", "description": "…" },
  "entityTypes": [ { "id": "…", "name": "…", "icon": "…", "color": "…",
                     "properties": [ { "key": "…", "label": "…", "dataType": "…", "config": {}, "required": false } ] } ],
  "relationTypes": [ { "id": "…", "name": "…", "inverseName": "…", "directionality": "directed" } ],
  "entities": [ { "id": "…", "typeId": "…", "name": "…", "summary": "…", "properties": {}, "bodyJson": {}, "createdAt": "…", "updatedAt": "…" } ],
  "relations": [ { "typeId": "…", "sourceId": "…", "targetId": "…", "description": "…", "startDate": "…", "endDate": "…", "propertyKey": null } ]
}
```

- Os ids do arquivo só servem para ligar as partes entre si: **na importação todos são regerados** e remapeados (referências, menções, relações e restrições de tipo).
- `formatVersion` sobe apenas em mudanças incompatíveis. Um arquivo de versão mais nova é recusado.

## Exportação em Markdown

`<Universo>/<Tipo>/<Entidade>.md` e `_Índice.md`. Cada nota tem frontmatter (`uwb_id`, `uwb_type`, `uwb_created`, `uwb_updated` e as propriedades), o resumo como citação, o documento e uma seção **Relações**. Referências viram `"[[Nome]]"`. Os nomes de arquivo são únicos no universo todo para que os links nunca fiquem ambíguos.