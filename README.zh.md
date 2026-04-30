# Soffio

[![CI](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml)
[![Release](https://github.com/xfyyzy/soffio/actions/workflows/release.yml/badge.svg)](https://github.com/xfyyzy/soffio/actions/workflows/release.yml)
[![Rust Edition](https://img.shields.io/badge/Rust%20Edition-2024-orange?logo=rust&logoColor=white)](https://doc.rust-lang.org/edition-guide/)
[![公开站点](https://img.shields.io/website?url=https%3A%2F%2Fsoffio.xfyyzy.xyz&label=%E5%85%AC%E5%BC%80%E7%AB%99%E7%82%B9)](https://soffio.xfyyzy.xyz)
[![管理站点](https://img.shields.io/website?url=https%3A%2F%2Fadmin.soffio.xfyyzy.xyz&label=%E7%AE%A1%E7%90%86%E7%AB%99%E7%82%B9)](https://admin.soffio.xfyyzy.xyz)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/xfyyzy/soffio)

[English](README.md) | 中文

Soffio 是一套克制的自托管发布系统，面向希望兼得静态输出、管理便利与运维控制权的技术写作者。

Soffio 不是通用 CMS。它服务于那些想写作、发布、自动化和自托管，又不愿交出控制权的人。它偏好静态输出而不是运行时魔法，偏好朴素可靠而不是插件蔓延，偏好显式工作流而不是隐藏自动化。

公开站点以静态渲染为主，读者侧交互保持服务端驱动；管理站点聚焦写作、编辑、自动化和发布流程。核心由 Rust、Axum、Askama、SQLx 构成。
演示站点（整点重置）：
- 公开站点：<https://soffio.xfyyzy.xyz>
- 管理站点：<https://admin.soffio.xfyyzy.xyz>

> 本仓库采用 2-clause BSD 许可（参见 `LICENSE`）。


## 架构速览

```
src/
├── domain        # 领域模型与业务不变量
├── application   # 用例服务、仓库接口、作业调度
├── infra         # Postgres 仓库、HTTP 适配层、遥测
├── presentation  # 视图模型、模板与布局
├── util          # 附属工具（时区、标识符等）
└── main.rs       # CLI/服务入口
```

核心约束详见 `AGENTS.md`：领域层保持纯函数，副作用隔离在基础设施层。

## 快速开始

1. **准备依赖**
    - Rust 稳定版 >= 1.91（支持 2024 Edition）
    - PostgreSQL 18（默认连接 `postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`）
    - TypeScript Compiler 6.x（`tsc --version`，当前验证版本 6.0.2）

2. **启动服务**
   ```bash
   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev cargo run --bin soffio
   ```
    - 公共站点默认监听 `127.0.0.1:3000`
    - 管理站点默认监听 `127.0.0.1:3001`
    - 可通过 CLI 参数或环境变量覆盖



## 运行时组件

- **HTTP 服务**：Axum 8，采用双监听地址区分公开与管理面；公共路由在 `src/infra/http/public.rs`，管理路由在
  `src/infra/http/admin/`。
- **数据库访问**：SQLx Postgres，所有仓库集中在 `src/infra/db`，领域接口定义见 `src/application/repos.rs`。
- **日志与追踪**：`tracing`, `tracing-subscriber`，统一入口 `src/infra/telemetry.rs`。

## 无头 API

- 基础路径：公共监听上的 `/api/v1`。
- 认证：`Authorization: Bearer <api_key>`，密钥仅在管理后台的“API keys”页面展示一次，操作指南见 [`docs/admin/api-keys.md`](docs/admin/api-keys.md)。
- 权限：通过 scope 控制（蛇形命名）：`post_read`, `post_write`, `page_read`, `page_write`, `tag_read`, `tag_write`, `navigation_read`, `navigation_write`, `upload_read`, `upload_write`, `settings_read`, `settings_write`, `job_read`, `audit_read`。
- 限流：独立配置 `api_rate_limit`（默认 60 秒内每密钥 120 次）。
- 规范：参见 [`docs/api/openapi.yaml`](docs/api/openapi.yaml)。

## soffio-cli

Headless API 的命令行客户端。生成式命令矩阵与使用指南见 [`docs/cli.zh.md`](docs/cli.zh.md)。

快速开始：

```
cargo build -p soffio-cli --release
SOFFIO_SITE_URL=https://your.site \
SOFFIO_API_KEY_FILE=~/.config/soffio/key \
./target/release/soffio-cli api-keys me
```

从文件创建文章示例：

```
./target/release/soffio-cli posts create \
  --title "标题" --excerpt "摘要" \
  --body-file ./post.md --summary-file ./post.summary.md \
  --status published
```

## 开发工作流

1. 运行分层质量门禁：
   ```bash
   # 请配置可写数据库；SQLX_TEST_DATABASE_URL 供 `#[sqlx::test]` 创建临时库
   export DATABASE_URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev
   export SQLX_TEST_DATABASE_URL=postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres

   # 日常快速反馈
   ./scripts/gate-fast.sh

   # 提交 PR / 合并前；启动本地 Postgres、导入 seed、渲染派生内容、
   # 启动临时 soffio 实例、运行 full gate，最后停止该实例
   ./scripts/gate-full-local.sh

   # 仅在已手动准备数据库和本地 soffio 实例时使用
   ./scripts/gate-full.sh

   # 周期性依赖体检（例如每周）
   ./scripts/gate-hygiene.sh
   ```
2. 如果不使用 `gate-full-local.sh`，请手动准备 full gate 前置条件：
   ```bash
   docker compose -f docker-compose-dev.yml up -d

   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \
     cargo run --bin soffio -- import seed/seed.toml

   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \
     cargo run --bin soffio -- renderall

   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \
     target/debug/soffio serve
   ```
   当默认本地 Postgres 未就绪，或 `tests/api_keys.seed.toml` 指向的 seeded API 不可用时，`gate-full.sh`
   会提前失败并打印修复命令。`SKIP_LIVE_TESTS=1` 仅用于非发布诊断。
3. 参考 `CONTRIBUTING.md` 获取分支策略与提交要求。
4. 提交 PR 时遵循 `.github/PULL_REQUEST_TEMPLATE.md`，并保持 CI 绿色。

## 部署

生产环境部署指南：

- **Docker 部署**：参见 [`docs/deploy/docker.md`](docs/deploy/docker.md)

## 发布说明

发布记录维护在 `CHANGELOG.md`。每个 Release 请附带：

1. 迁移脚本与数据兼容性说明
2. 新增配置项及默认值变更
3. 发布二进制产物时，同时附带 Linux musl、FreeBSD 15 x86_64 与 Darwin aarch64 压缩包

## 支持与安全

- 常见问题与支持渠道：`SUPPORT.md`
- 漏洞披露流程：`SECURITY.md`
- 社区行为准则：`CODE_OF_CONDUCT.md`

## 许可证

`BSD-2-Clause` — 详情参见根目录下的 `LICENSE`。
