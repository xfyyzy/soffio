-- API keys support

CREATE TYPE api_scope AS ENUM (
    'content_read',
    'content_write',
    'tag_write',
    'navigation_write',
    'upload_write',
    'settings_write',
    'jobs_read',
    'audit_read'
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    prefix TEXT NOT NULL UNIQUE,
    hashed_secret BYTEA NOT NULL,
    scopes api_scope[] NOT NULL,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT api_keys_scopes_not_empty CHECK (array_length(scopes, 1) > 0)
);

CREATE INDEX api_keys_active_idx
    ON api_keys (revoked_at, expires_at);

CREATE INDEX api_keys_last_used_idx
    ON api_keys (last_used_at);
