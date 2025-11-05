-- Consolidated schema migration

CREATE TYPE post_status AS ENUM ('draft', 'published', 'archived', 'error');
CREATE TYPE page_status AS ENUM ('draft', 'published', 'archived', 'error');
CREATE TYPE navigation_destination_type AS ENUM ('internal', 'external');
CREATE TYPE job_status AS ENUM ('queued', 'running', 'failed', 'completed');
CREATE TYPE job_type AS ENUM ('render_post', 'render_page', 'render_summary');

CREATE TABLE posts (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    excerpt TEXT NOT NULL,
    body_markdown TEXT NOT NULL,
    status post_status NOT NULL DEFAULT 'draft',
    pinned BOOLEAN NOT NULL DEFAULT FALSE,
    scheduled_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    summary_markdown TEXT,
    summary_html TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT posts_published_requires_timestamp
        CHECK (status <> 'published' OR published_at IS NOT NULL)
);

CREATE INDEX posts_published_keyset_idx
    ON posts (pinned DESC, published_at DESC NULLS LAST, id DESC)
    WHERE status = 'published';

CREATE INDEX posts_admin_primary_time_idx
    ON posts (
        pinned DESC,
        (CASE
            WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
            ELSE COALESCE(updated_at, created_at)
        END) DESC,
        id DESC
    )
    INCLUDE (status);

CREATE INDEX posts_primary_time_idx
    ON posts (
        (CASE
            WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
            ELSE COALESCE(updated_at, created_at)
        END) DESC,
        id DESC
    );

CREATE TABLE post_sections (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES post_sections(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    level SMALLINT NOT NULL,
    heading_html TEXT NOT NULL,
    heading_text TEXT NOT NULL,
    body_html TEXT NOT NULL,
    contains_code BOOLEAN NOT NULL DEFAULT FALSE,
    anchor_slug TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX post_sections_root_position_unique
    ON post_sections (post_id, position)
    WHERE parent_id IS NULL;

CREATE UNIQUE INDEX post_sections_child_position_unique
    ON post_sections (post_id, parent_id, position)
    WHERE parent_id IS NOT NULL;

CREATE INDEX post_sections_listing_idx
    ON post_sections (post_id, parent_id, position);

CREATE TABLE pages (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    body_markdown TEXT NOT NULL,
    rendered_html TEXT NOT NULL,
    status page_status NOT NULL DEFAULT 'draft',
    scheduled_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT pages_published_requires_timestamp
        CHECK (status <> 'published' OR published_at IS NOT NULL)
);

CREATE INDEX pages_published_keyset_idx
    ON pages (published_at DESC NULLS LAST, id DESC)
    WHERE status = 'published';

CREATE INDEX pages_primary_time_idx
    ON pages (
        (CASE
            WHEN status = 'published'::page_status THEN COALESCE(published_at, updated_at, created_at)
            ELSE COALESCE(updated_at, created_at)
        END) DESC,
        id DESC
    )
    INCLUDE (status);

CREATE TABLE tags (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    pinned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_tags_pinned_name
    ON tags (pinned DESC, LOWER(name), slug);

CREATE INDEX idx_tags_pinned_primary_time
    ON tags (pinned DESC, COALESCE(updated_at, created_at) DESC, id DESC);

CREATE TABLE post_tags (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

CREATE INDEX post_tags_tag_id_idx
    ON post_tags (tag_id);

CREATE TABLE navigation_items (
    id UUID PRIMARY KEY,
    label TEXT NOT NULL,
    destination_type navigation_destination_type NOT NULL,
    destination_url TEXT,
    destination_page_id UUID REFERENCES pages(id) ON DELETE RESTRICT,
    sort_order INTEGER NOT NULL,
    open_in_new_tab BOOLEAN NOT NULL DEFAULT FALSE,
    visible BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT navigation_destination_requirements CHECK (
        (destination_type = 'internal' AND destination_page_id IS NOT NULL AND destination_url IS NULL) OR
        (destination_type = 'external' AND destination_url IS NOT NULL AND destination_page_id IS NULL)
    )
);

CREATE INDEX navigation_items_order_idx
    ON navigation_items (
        sort_order ASC,
        COALESCE(updated_at, created_at) DESC,
        id ASC
    );

CREATE OR REPLACE FUNCTION ensure_navigation_internal_page_published()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.destination_type = 'internal' THEN
        IF NEW.destination_page_id IS NULL THEN
            RAISE EXCEPTION 'internal navigation item must provide destination_page_id';
        END IF;

        PERFORM 1
        FROM pages p
        WHERE p.id = NEW.destination_page_id
          AND p.status = 'published'::page_status
          AND p.published_at IS NOT NULL;

        IF NOT FOUND THEN
            RAISE EXCEPTION 'navigation internal link must reference a published page (id=%)', NEW.destination_page_id;
        END IF;

        NEW.destination_url := NULL;
    ELSE
        NEW.destination_page_id := NULL;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER navigation_internal_requires_published_page
    BEFORE INSERT OR UPDATE ON navigation_items
    FOR EACH ROW
    EXECUTE FUNCTION ensure_navigation_internal_page_published();

CREATE TABLE uploads (
    id UUID PRIMARY KEY,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uploads_checksum_unique UNIQUE (checksum)
);

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    payload_text TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_logs_cursor_idx
    ON audit_logs (created_at DESC, id DESC);

CREATE TABLE jobs (
    id UUID PRIMARY KEY,
    job_type job_type NOT NULL,
    payload JSONB NOT NULL,
    status job_status NOT NULL DEFAULT 'queued',
    error_text TEXT,
    scheduled_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX jobs_status_cursor_idx
    ON jobs (status, scheduled_at DESC NULLS LAST, id DESC);

CREATE OR REPLACE FUNCTION app_is_valid_timezone(name text)
RETURNS boolean
LANGUAGE sql
STABLE
STRICT
AS $$
SELECT EXISTS (
    SELECT 1
    FROM pg_catalog.pg_timezone_names
    WHERE name = $1
);
$$;

CREATE TABLE site_settings (
    id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    homepage_size INTEGER NOT NULL DEFAULT 6,
    show_tag_aggregations BOOLEAN NOT NULL DEFAULT TRUE,
    show_month_aggregations BOOLEAN NOT NULL DEFAULT TRUE,
    global_toc_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    brand_title TEXT NOT NULL DEFAULT 'Soffio',
    brand_href TEXT NOT NULL DEFAULT '/',
    footer_copy TEXT NOT NULL DEFAULT 'Stillness guides the wind; the wind reshapes stillness.',
    meta_title TEXT NOT NULL DEFAULT 'Soffio',
    meta_description TEXT NOT NULL DEFAULT 'Whispers on motion, balance, and form.',
    og_title TEXT NOT NULL DEFAULT 'Soffio',
    og_description TEXT NOT NULL DEFAULT 'Traces of motion, balance, and form in continual drift.',
    admin_page_size INTEGER NOT NULL DEFAULT 6,
    public_site_url TEXT NOT NULL DEFAULT 'http://localhost:3000/',
    timezone TEXT NOT NULL DEFAULT 'Asia/Shanghai',
    favicon_svg TEXT NOT NULL DEFAULT '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"></svg>',
    tag_filter_limit INTEGER NOT NULL DEFAULT 16,
    month_filter_limit INTEGER NOT NULL DEFAULT 16
);

ALTER TABLE site_settings
    ADD CONSTRAINT site_settings_timezone_valid
    CHECK (app_is_valid_timezone(timezone));

INSERT INTO site_settings DEFAULT VALUES
ON CONFLICT (id) DO NOTHING;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE SCHEMA IF NOT EXISTS apalis;

CREATE TABLE apalis.workers (
    id TEXT NOT NULL,
    worker_type TEXT NOT NULL,
    storage_name TEXT NOT NULL,
    layers TEXT NOT NULL DEFAULT '',
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS unique_worker_id ON apalis.workers (id);
CREATE INDEX IF NOT EXISTS WTIdx ON apalis.workers(worker_type);
CREATE INDEX IF NOT EXISTS LSIdx ON apalis.workers(last_seen);

CREATE TABLE apalis.jobs (
    job JSONB NOT NULL,
    id TEXT NOT NULL,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 25,
    run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error TEXT,
    lock_at TIMESTAMPTZ,
    lock_by TEXT,
    done_at TIMESTAMPTZ,
    priority INTEGER DEFAULT 0,
    CONSTRAINT fk_worker_lock_by FOREIGN KEY (lock_by) REFERENCES apalis.workers(id)
);

CREATE INDEX IF NOT EXISTS SIdx ON apalis.jobs(status);
CREATE UNIQUE INDEX IF NOT EXISTS unique_job_id ON apalis.jobs (id);
CREATE INDEX IF NOT EXISTS LIdx ON apalis.jobs(lock_by);
CREATE INDEX IF NOT EXISTS JTIdx ON apalis.jobs(job_type);
CREATE INDEX IF NOT EXISTS apalis_jobs_schedule_idx
    ON apalis.jobs (run_at DESC, id DESC);

CREATE OR REPLACE FUNCTION generate_ulid()
RETURNS TEXT
AS $$
DECLARE
  encoding   BYTEA = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  timestamp  BYTEA = E'\\000\\000\\000\\000\\000\\000';
  output     TEXT = '';

  unix_time  BIGINT;
  ulid       BYTEA;
BEGIN
  unix_time = (EXTRACT(EPOCH FROM CLOCK_TIMESTAMP()) * 1000)::BIGINT;
  timestamp = SET_BYTE(timestamp, 0, (unix_time >> 40)::BIT(8)::INTEGER);
  timestamp = SET_BYTE(timestamp, 1, (unix_time >> 32)::BIT(8)::INTEGER);
  timestamp = SET_BYTE(timestamp, 2, (unix_time >> 24)::BIT(8)::INTEGER);
  timestamp = SET_BYTE(timestamp, 3, (unix_time >> 16)::BIT(8)::INTEGER);
  timestamp = SET_BYTE(timestamp, 4, (unix_time >> 8)::BIT(8)::INTEGER);
  timestamp = SET_BYTE(timestamp, 5, unix_time::BIT(8)::INTEGER);

  ulid = timestamp || gen_random_bytes(10);

  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 0) & 224) >> 5));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 0) & 31)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 1) & 248) >> 3));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 1) & 7) << 2) | ((GET_BYTE(ulid, 2) & 192) >> 6)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 2) & 62) >> 1));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 2) & 1) << 4) | ((GET_BYTE(ulid, 3) & 240) >> 4)));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 3) & 15) << 1) | ((GET_BYTE(ulid, 4) & 128) >> 7)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 4) & 124) >> 2));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 4) & 3) << 3) | ((GET_BYTE(ulid, 5) & 224) >> 5)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 5) & 31)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 6) & 248) >> 3));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 6) & 7) << 2) | ((GET_BYTE(ulid, 7) & 192) >> 6)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 7) & 62) >> 1));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 7) & 1) << 4) | ((GET_BYTE(ulid, 8) & 240) >> 4)));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 8) & 15) << 1) | ((GET_BYTE(ulid, 9) & 128) >> 7)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 9) & 124) >> 2));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 9) & 3) << 3) | ((GET_BYTE(ulid, 10) & 224) >> 5)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 10) & 31)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 11) & 248) >> 3));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 11) & 7) << 2) | ((GET_BYTE(ulid, 12) & 192) >> 6)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 12) & 62) >> 1));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 12) & 1) << 4) | ((GET_BYTE(ulid, 13) & 240) >> 4)));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 13) & 15) << 1) | ((GET_BYTE(ulid, 14) & 128) >> 7)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 14) & 124) >> 2));
  output = output || CHR(GET_BYTE(encoding, ((GET_BYTE(ulid, 14) & 3) << 3) | ((GET_BYTE(ulid, 15) & 224) >> 5)));
  output = output || CHR(GET_BYTE(encoding, (GET_BYTE(ulid, 15) & 31)));

  RETURN output;
END
$$
LANGUAGE plpgsql
VOLATILE;

CREATE OR REPLACE FUNCTION apalis.push_job(
    job_type TEXT,
    job JSON DEFAULT NULL :: json,
    status TEXT DEFAULT 'Pending' :: text,
    run_at TIMESTAMPTZ DEFAULT now() :: timestamptz,
    max_attempts INTEGER DEFAULT 25 :: integer,
    priority INTEGER DEFAULT 0 :: integer
) RETURNS apalis.jobs AS $$
DECLARE
    v_job_row apalis.jobs;
    v_job_id TEXT;
BEGIN
    IF job_type IS NOT NULL AND length(job_type) > 512 THEN
        RAISE EXCEPTION 'Job_type is too long (max length: 512).' USING errcode = 'APAJT';
    END IF;

    IF max_attempts < 1 THEN
        RAISE EXCEPTION 'Job maximum attempts must be at least 1.' USING errcode = 'APAMA';
    END IF;

    SELECT generate_ulid() INTO v_job_id;

    INSERT INTO apalis.jobs (
        job,
        id,
        job_type,
        status,
        attempts,
        max_attempts,
        run_at,
        last_error,
        lock_at,
        lock_by,
        done_at,
        priority
    )
    VALUES (
        job,
        v_job_id,
        job_type,
        status,
        0,
        max_attempts,
        run_at,
        NULL,
        NULL,
        NULL,
        NULL,
        priority
    )
    RETURNING * INTO v_job_row;

    RETURN v_job_row;
END;
$$ LANGUAGE plpgsql VOLATILE;

CREATE OR REPLACE FUNCTION apalis.get_jobs(
    worker_id TEXT,
    v_job_type TEXT,
    v_job_count INTEGER DEFAULT 5 :: integer
) RETURNS SETOF apalis.jobs AS $$
BEGIN
    RETURN QUERY
    UPDATE apalis.jobs
    SET status = 'Running',
        lock_by = worker_id,
        lock_at = now()
    WHERE id IN (
        SELECT id
        FROM apalis.jobs
        WHERE (status = 'Pending' OR (status = 'Failed' AND attempts < max_attempts))
          AND run_at < now()
          AND job_type = v_job_type
        ORDER BY priority DESC, run_at ASC
        LIMIT v_job_count
        FOR UPDATE SKIP LOCKED
    )
    RETURNING *;
END;
$$ LANGUAGE plpgsql VOLATILE;

CREATE OR REPLACE FUNCTION apalis.notify_new_jobs()
RETURNS trigger AS $$
BEGIN
     PERFORM pg_notify('apalis::job', 'insert');
     RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notify_workers
    AFTER INSERT ON apalis.jobs
    FOR EACH STATEMENT
    EXECUTE FUNCTION apalis.notify_new_jobs();
