# soffio-cli

[English](cli.md) | 中文

Soffio 的 headless 管理 CLI，覆盖帖子、页面、标签、导航、上传、站点设置、后台任务、审计等管理场景。

## 快速开始
```
cargo build --release --bin soffio-cli
SOFFIO_SITE_URL=https://your.site \
SOFFIO_API_KEY_FILE=~/.config/soffio/key \
./target/release/soffio-cli api-keys me
```

## 全局选项
- `--site`（或环境变量 `SOFFIO_SITE_URL`），必填。
- `--key-file`（或 `SOFFIO_API_KEY_FILE`），文件优先。
- API 密钥只能通过环境变量 `SOFFIO_API_KEY` 提供，出于安全考虑不提供 CLI 旗标。
- 所有命令都支持 `--help` / `--version`。

## 长文本输入
通过文件参数避免转义问题：`--body-file`、`--summary-file`、`--description-file`、`--favicon-svg-file`，文件内容将被原样读取。

## 命令矩阵（自动生成）
| Command | Description |
|---|---|
| `soffio-cli` | Soffio headless API CLI |
| `soffio-cli api-keys` | API key inspection |
| `soffio-cli api-keys me` | Show current API key metadata/scopes |
| `soffio-cli posts` | Post management (list/read/write/status/tags) |
| `soffio-cli posts list` | List posts with optional filters |
| `soffio-cli posts get` | Get a post by id or slug |
| `soffio-cli posts create` | Create a post |
| `soffio-cli posts update` | Update all mutable fields of a post |
| `soffio-cli posts patch-title` | Patch title only |
| `soffio-cli posts patch-excerpt` | Patch excerpt |
| `soffio-cli posts patch-body` | Patch body (supports file input) |
| `soffio-cli posts patch-summary` | Patch summary (supports file input) |
| `soffio-cli posts status` | Update status and schedule times |
| `soffio-cli posts tags` | Replace tag list |
| `soffio-cli posts pin` | Pin or unpin |
| `soffio-cli posts delete` | Delete a post |
| `soffio-cli pages` | Page management |
| `soffio-cli pages list` | List pages |
| `soffio-cli pages get` | Get a page by id or slug |
| `soffio-cli pages create` | Create a page |
| `soffio-cli pages update` | Update a page |
| `soffio-cli pages patch-title` | Patch title only |
| `soffio-cli pages patch-body` | Patch body |
| `soffio-cli pages status` | Update status and times |
| `soffio-cli pages delete` | Delete a page |
| `soffio-cli tags` | Tag management |
| `soffio-cli tags list` | List tags |
| `soffio-cli tags get` | Get a tag by id or slug |
| `soffio-cli tags create` | Create a tag |
| `soffio-cli tags update` | Update all fields |
| `soffio-cli tags patch-pin` | Pin or unpin |
| `soffio-cli tags patch-name` | Update name only |
| `soffio-cli tags patch-description` | Update description only (supports file input) |
| `soffio-cli tags delete` | Delete a tag |
| `soffio-cli navigation` | Navigation menu management |
| `soffio-cli navigation list` | List navigation items |
| `soffio-cli navigation get` | Get a navigation item by id |
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
| `soffio-cli uploads get` | Get an upload by id |
| `soffio-cli uploads upload` | Upload a file |
| `soffio-cli uploads delete` | Delete an upload |
| `soffio-cli settings` | Site-wide settings |
| `soffio-cli settings get` | Show settings |
| `soffio-cli settings patch` | Patch settings (only provided fields) |
| `soffio-cli jobs` | Background jobs |
| `soffio-cli jobs list` | List background jobs |
| `soffio-cli audit` | Audit log access |
| `soffio-cli audit list` | List audit logs |
| `soffio-cli snapshots` | Snapshots management |
| `soffio-cli snapshots list` | List snapshots |
| `soffio-cli snapshots get` | Get a snapshot |
| `soffio-cli snapshots create` | Create a snapshot |
| `soffio-cli snapshots rollback` | Rollback to a snapshot |


## 常用场景示例
- 查看当前密钥信息：`soffio-cli api-keys me`
- 从文件创建文章：`soffio-cli posts create --title "标题" --excerpt "摘要" --body-file post.md --summary-file summary.md --status published`
- 更新页面正文：`soffio-cli pages patch-body --id <UUID> --body-file page.md`
- 上传资源：`soffio-cli uploads upload ./image.png`
- 按状态查看任务：`soffio-cli jobs list --state running`
- 检索审计日志：`soffio-cli audit list --actor admin --action delete_post`

## 安全提示
- 使用密钥文件 / 环境变量，避免在命令行暴露密钥。
- CLI 无状态，如遇网络错误可重试；输出为 JSON，便于 `jq` 等工具处理。
