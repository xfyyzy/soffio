CREATE INDEX uploads_created_at_id_idx
    ON uploads (created_at DESC, id DESC);

CREATE INDEX uploads_content_type_created_at_id_idx
    ON uploads (content_type, created_at DESC, id DESC);
