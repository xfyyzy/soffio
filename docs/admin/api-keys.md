# API keys (headless API)

- Manage keys in the admin console: **API keys** nav item.
- Keys are shown only once on creation/rotation — copy immediately and store securely.
- Scopes:
  - `content_read` / `content_write` — posts/pages CRUD.
  - `tag_write` — tag CRUD.
  - `navigation_write` — navigation CRUD.
  - `upload_write` — file uploads and deletions.
  - `settings_write` — patch site settings.
  - `jobs_read` — read job queue.
  - `audit_read` — read audit log.
- Tokens use the format `sk_<prefix>_<secret>`; the prefix is logged for observability while the secret is never stored.
- Revoke keys immediately when compromised; rotate to issue a replacement and invalidate the old secret in one step.
- All API endpoints are rate-limited separately from the public site (see `api_rate_limit` in [`soffio.toml.example`](../../soffio.toml.example)).
- Full OpenAPI contract: [`docs/api/openapi.yaml`](../api/openapi.yaml).
