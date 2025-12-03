# soffio-cli

English | [中文](cli.zh.md)

A headless admin CLI for Soffio. Built to cover every admin scenario (posts, pages, tags, navigation, uploads, settings, jobs, audit, API key introspection) via the headless HTTP API.

## Quick start
```
cargo build --release --bin soffio-cli
SOFFIO_SITE_URL=https://your.site \
SOFFIO_API_KEY_FILE=~/.config/soffio/key \
./target/release/soffio-cli api-keys me
```

## Global options
- `--site` (env `SOFFIO_SITE_URL`), required.
- `--key-file` (env `SOFFIO_API_KEY_FILE`), file wins over env.
- API key is **env only** (`SOFFIO_API_KEY`); no CLI flag to avoid shell history leaks.
- `--help` / `--version` available everywhere.

## Long text input
Use file flags to avoid shell quoting limits: `--body-file`, `--summary-file`, `--description-file`, `--favicon-svg-file`. The file content is read verbatim.

## Command matrix (generated)
| Command | Description |
|---|---|
| `soffio-cli` | Soffio headless API CLI |
| `soffio-cli api-keys` | API key inspection |
| `soffio-cli api-keys me` | Show current API key metadata/scopes |
| `soffio-cli posts` | Post management (list/read/write/status/tags) |
| `soffio-cli posts list` | List posts with optional filters |
| `soffio-cli posts get` | Get a post by slug |
| `soffio-cli posts create` | Create a post |
| `soffio-cli posts update` | Update all mutable fields of a post |
| `soffio-cli posts patch-title-slug` | Patch only title and slug |
| `soffio-cli posts patch-excerpt` | Patch excerpt |
| `soffio-cli posts patch-body` | Patch body (supports file input) |
| `soffio-cli posts patch-summary` | Patch summary (supports file input) |
| `soffio-cli posts status` | Update status and schedule times |
| `soffio-cli posts tags` | Replace tag list |
| `soffio-cli posts pin` | Pin or unpin |
| `soffio-cli posts delete` | Delete a post |
| `soffio-cli pages` | Page management |
| `soffio-cli pages list` | List pages |
| `soffio-cli pages get` | Get by slug |
| `soffio-cli pages create` | Create a page |
| `soffio-cli pages update` | Update a page |
| `soffio-cli pages patch-title-slug` | Patch title/slug only |
| `soffio-cli pages patch-body` | Patch body |
| `soffio-cli pages status` | Update status and times |
| `soffio-cli pages delete` | Delete a page |
| `soffio-cli tags` | Tag management |
| `soffio-cli tags list` | List tags |
| `soffio-cli tags create` | Create a tag |
| `soffio-cli tags update` | Update all fields |
| `soffio-cli tags patch-pin` | Pin or unpin |
| `soffio-cli tags patch-name` | Update name only |
| `soffio-cli tags patch-description` | Update description only (supports file input) |
| `soffio-cli tags delete` | Delete a tag |
| `soffio-cli navigation` | Navigation menu management |
| `soffio-cli navigation list` | List navigation items |
| `soffio-cli navigation create` | Create a navigation entry |
| `soffio-cli navigation update` | Update all navigation fields |
| `soffio-cli navigation patch-label` | Patch label only |
| `soffio-cli navigation patch-destination` | Patch destination |
| `soffio-cli navigation patch-sort` | Patch sort order |
| `soffio-cli navigation patch-visibility` | Patch visibility |
| `soffio-cli navigation patch-open` | Patch open-in-new-tab flag |
| `soffio-cli navigation delete` | Delete a navigation entry |
| `soffio-cli uploads` | Asset uploads |
| `soffio-cli uploads list` | List uploads |
| `soffio-cli uploads upload` | Upload a file |
| `soffio-cli uploads delete` | Delete an upload |
| `soffio-cli settings` | Site-wide settings |
| `soffio-cli settings get` | Show settings |
| `soffio-cli settings patch` | Patch settings (only provided fields) |
| `soffio-cli jobs` | Background jobs |
| `soffio-cli jobs list` | List background jobs |
| `soffio-cli audit` | Audit log access |
| `soffio-cli audit list` | List audit logs |


## Typical admin scenarios
- Inspect current API key: `soffio-cli api-keys me`
- Create a post from files: `soffio-cli posts create --title "Title" --excerpt "Short" --body-file post.md --summary-file summary.md --status published`
- Patch a page body: `soffio-cli pages patch-body --id <UUID> --body-file page.md`
- Upload an asset: `soffio-cli uploads upload ./image.png`
- List jobs with filter: `soffio-cli jobs list --state queued --job-type send_email`
- Audit search: `soffio-cli audit list --actor alice --action update_post`

## Safety notes
- Prefer key files and env vars; never paste keys on the command line.
- CLI is stateless; retry on transport errors. Outputs are JSON pretty-printed for piping to `jq`.
