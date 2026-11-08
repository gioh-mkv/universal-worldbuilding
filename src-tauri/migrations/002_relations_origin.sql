-- Relações geradas a partir de propriedades entity_ref guardam a chave da propriedade.
-- NULL = relação criada manualmente pelo usuário.
ALTER TABLE relations ADD COLUMN property_key TEXT;
CREATE INDEX idx_rel_property ON relations(source_id, property_key);