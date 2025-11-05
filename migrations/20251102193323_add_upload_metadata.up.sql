ALTER TABLE uploads
    ADD COLUMN metadata JSONB NOT NULL DEFAULT '{}'::JSONB;

COMMENT ON COLUMN uploads.metadata IS 'Arbitrary per-upload metadata extracted at ingestion time.';
