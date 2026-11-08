CREATE TABLE universes (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  description TEXT,
  settings    TEXT NOT NULL DEFAULT '{}',
  created_at  TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE entity_types (
  id          TEXT PRIMARY KEY,
  universe_id TEXT NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,
  icon        TEXT,
  color       TEXT,
  description TEXT,
  position    INTEGER NOT NULL DEFAULT 0,
  UNIQUE (universe_id, name)
);

CREATE TABLE property_defs (
  id        TEXT PRIMARY KEY,
  type_id   TEXT NOT NULL REFERENCES entity_types(id) ON DELETE CASCADE,
  key       TEXT NOT NULL,
  label     TEXT NOT NULL,
  data_type TEXT NOT NULL CHECK (data_type IN
    ('text','number','date','datetime','boolean','select',
     'multiselect','image','file','entity_ref','coordinates','computed')),
  config    TEXT NOT NULL DEFAULT '{}',
  required  INTEGER NOT NULL DEFAULT 0,
  position  INTEGER NOT NULL DEFAULT 0,
  UNIQUE (type_id, key)
);

CREATE TABLE entities (
  id          TEXT PRIMARY KEY,
  universe_id TEXT NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
  type_id     TEXT NOT NULL REFERENCES entity_types(id),
  name        TEXT NOT NULL,
  summary     TEXT,
  properties  TEXT NOT NULL DEFAULT '{}',
  body_json   TEXT,
  body_md     TEXT,
  created_at  TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_entities_universe_type ON entities(universe_id, type_id);
CREATE INDEX idx_entities_name ON entities(universe_id, name);

CREATE TABLE relation_types (
  id             TEXT PRIMARY KEY,
  universe_id    TEXT NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
  name           TEXT NOT NULL,
  inverse_name   TEXT,
  directionality TEXT NOT NULL DEFAULT 'directed'
    CHECK (directionality IN ('directed','bidirectional','hierarchical')),
  UNIQUE (universe_id, name)
);

CREATE TABLE relations (
  id          TEXT PRIMARY KEY,
  universe_id TEXT NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
  type_id     TEXT NOT NULL REFERENCES relation_types(id),
  source_id   TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  target_id   TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  description TEXT,
  properties  TEXT NOT NULL DEFAULT '{}',
  start_date  TEXT,
  end_date    TEXT,
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_rel_source ON relations(source_id);
CREATE INDEX idx_rel_target ON relations(target_id);

CREATE TABLE mentions (
  source_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  target_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  PRIMARY KEY (source_id, target_id)
);

CREATE TABLE entity_versions (
  id         TEXT PRIMARY KEY,
  entity_id  TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  snapshot   TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE search_index USING fts5(
  entity_id UNINDEXED, universe_id UNINDEXED,
  name, summary, body, props,
  tokenize = 'unicode61 remove_diacritics 2'
);