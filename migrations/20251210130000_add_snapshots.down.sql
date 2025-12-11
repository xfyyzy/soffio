-- Rollback snapshots table and added scopes

-- Drop triggers and helper functions
DROP TRIGGER IF EXISTS trg_posts_delete_snapshots ON posts;
DROP TRIGGER IF EXISTS trg_pages_delete_snapshots ON pages;
DROP FUNCTION IF EXISTS delete_post_snapshots();
DROP FUNCTION IF EXISTS delete_page_snapshots();

DROP TABLE IF EXISTS snapshots;

DROP TYPE IF EXISTS snapshot_entity_type;

-- Remove added api_scope values by recreating type
ALTER TYPE api_scope RENAME TO api_scope_old;

CREATE TYPE api_scope AS ENUM (
    'post_read',
    'post_write',
    'page_read',
    'page_write',
    'tag_read',
    'tag_write',
    'navigation_read',
    'navigation_write',
    'upload_read',
    'upload_write',
    'settings_read',
    'settings_write',
    'job_read',
    'audit_read'
);

ALTER TABLE api_keys
    ALTER COLUMN scopes TYPE api_scope[] USING scopes::text::api_scope[];

DROP TYPE api_scope_old;
