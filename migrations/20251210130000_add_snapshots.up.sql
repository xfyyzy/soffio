-- Snapshot system: shared snapshots table and scopes

-- Enumerates supported snapshot entity types
CREATE TYPE snapshot_entity_type AS ENUM ('post', 'page');

-- Extend API scopes for snapshot operations
ALTER TYPE api_scope ADD VALUE IF NOT EXISTS 'snapshot_read';
ALTER TYPE api_scope ADD VALUE IF NOT EXISTS 'snapshot_write';

CREATE TABLE snapshots (
    id UUID PRIMARY KEY,
    entity_type snapshot_entity_type NOT NULL,
    entity_id UUID NOT NULL,
    version INTEGER NOT NULL,
    description TEXT,
    schema_version BIGINT NOT NULL,
    content JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT snapshots_entity_version_unique UNIQUE (entity_type, entity_id, version)
);

CREATE INDEX idx_snapshots_entity ON snapshots(entity_type, entity_id, created_at DESC);
CREATE INDEX idx_snapshots_created_at ON snapshots(created_at DESC);

-- DB-level orphan prevention via delete triggers on concrete entity tables
CREATE OR REPLACE FUNCTION delete_post_snapshots() RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM snapshots WHERE entity_type = 'post' AND entity_id = OLD.id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_posts_delete_snapshots
AFTER DELETE ON posts
FOR EACH ROW EXECUTE FUNCTION delete_post_snapshots();

CREATE OR REPLACE FUNCTION delete_page_snapshots() RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM snapshots WHERE entity_type = 'page' AND entity_id = OLD.id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_pages_delete_snapshots
AFTER DELETE ON pages
FOR EACH ROW EXECUTE FUNCTION delete_page_snapshots();
