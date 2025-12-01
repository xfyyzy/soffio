# API keys (headless API)

- Manage keys in the admin console: **API keys** nav item.
- Keys are shown only once on creation/rotation — copy immediately and store securely.
- Scopes（snake_case，与实现一致）:
  - Posts: `post_read`, `post_write`
  - Pages: `page_read`, `page_write`
  - Tags: `tag_read`, `tag_write`
  - Navigation: `navigation_read`, `navigation_write`
  - Uploads: `upload_read`, `upload_write`
  - Settings: `settings_read`, `settings_write`
  - Jobs: `job_read`
  - Audit log: `audit_read`
- Tokens use the format `sk_<prefix>_<secret>`; the prefix is logged for observability while the secret is never stored.
- Revoke keys immediately when compromised; rotate to issue a replacement and invalidate the old secret in one step.
- All API endpoints are rate-limited separately from the public site (see `api_rate_limit` in [`soffio.toml.example`](../../soffio.toml.example)).
- Full OpenAPI contract: [`docs/api/openapi.yaml`](../api/openapi.yaml).
