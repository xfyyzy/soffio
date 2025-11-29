-- API keys support

-- Status enum for explicit key state tracking
CREATE TYPE api_key_status AS ENUM (
    'active',
    'revoked',
    'expired'
);

-- Redesigned scope enum with proper domain/action granularity
CREATE TYPE api_scope AS ENUM (
    -- Posts (was content_read/write)
    'post_read',
    'post_write',
    -- Pages (split from content)
    'page_read',
    'page_write',
    -- Tags (add read)
    'tag_read',
    'tag_write',
    -- Navigation (add read)
    'navigation_read',
    'navigation_write',
    -- Uploads (add read)
    'upload_read',
    'upload_write',
    -- Settings (add read)
    'settings_read',
    'settings_write',
    -- Jobs (rename for consistency)
    'job_read',
    -- Audit
    'audit_read'
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    prefix TEXT NOT NULL UNIQUE,
    hashed_secret BYTEA NOT NULL,
    scopes api_scope[] NOT NULL,
    status api_key_status NOT NULL DEFAULT 'active',
    expires_in INTERVAL,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT api_keys_scopes_not_empty CHECK (array_length(scopes, 1) > 0)
);

-- Index for status filtering (critical for list page tabs)
CREATE INDEX idx_api_keys_status ON api_keys(status);

CREATE INDEX api_keys_last_used_idx
    ON api_keys (last_used_at);
