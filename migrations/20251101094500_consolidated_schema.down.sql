-- Revert consolidated schema

DROP TRIGGER IF EXISTS navigation_internal_requires_published_page ON navigation_items;
DROP FUNCTION IF EXISTS ensure_navigation_internal_page_published();

DROP TRIGGER IF EXISTS notify_workers ON apalis.jobs;
DROP FUNCTION IF EXISTS apalis.notify_new_jobs();
DROP FUNCTION IF EXISTS apalis.get_jobs(TEXT, TEXT, INTEGER);
DROP FUNCTION IF EXISTS apalis.push_job(TEXT, JSON, TEXT, TIMESTAMPTZ, INTEGER, INTEGER);
DROP FUNCTION IF EXISTS generate_ulid();

DROP TABLE IF EXISTS apalis.jobs;
DROP TABLE IF EXISTS apalis.workers;
DROP SCHEMA IF EXISTS apalis;
DROP EXTENSION IF EXISTS pgcrypto;

DROP TABLE IF EXISTS post_tags;
DROP TABLE IF EXISTS post_sections;
DROP TABLE IF EXISTS navigation_items;
DROP TABLE IF EXISTS pages;
DROP TABLE IF EXISTS posts;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS site_settings;
DROP FUNCTION IF EXISTS app_is_valid_timezone(TEXT);
DROP TABLE IF EXISTS uploads;
DROP TABLE IF EXISTS audit_logs;
DROP TABLE IF EXISTS jobs;

DROP TYPE IF EXISTS job_type;
DROP TYPE IF EXISTS job_status;
DROP TYPE IF EXISTS navigation_destination_type;
DROP TYPE IF EXISTS page_status;
DROP TYPE IF EXISTS post_status;
