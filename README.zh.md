# Soffio

[![CI](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml)
[![Release](https://github.com/xfyyzy/soffio/actions/workflows/release.yml/badge.svg)](https://github.com/xfyyzy/soffio/actions/workflows/release.yml)
[![Rust Edition](https://img.shields.io/badge/Rust%20Edition-2024-orange?logo=rust&logoColor=white)](https://doc.rust-lang.org/edition-guide/)
[![公开站点](https://img.shields.io/website?url=https%3A%2F%2Fsoffio.xfyyzy.xyz&label=%E5%85%AC%E5%BC%80%E7%AB%99%E7%82%B9)](https://soffio.xfyyzy.xyz)
[![管理站点](https://img.shields.io/website?url=https%3A%2F%2Fadmin.soffio.xfyyzy.xyz&label=%E7%AE%A1%E7%90%86%E7%AB%99%E7%82%B9)](https://admin.soffio.xfyyzy.xyz)

<p align="center">
  <a href="https://www.producthunt.com/products/soffio?embed=true&utm_source=badge-featured&utm_medium=badge&utm_source=badge-soffio" target="_blank"><img src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1037444&theme=light&t=1763008766261" alt="Soffio - Rust&#0045;native&#0032;publishing&#0032;with&#0032;a&#0032;calm&#0044;&#0032;focused&#0032;admin | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" /></a>
</p>

[English](README.md) | 中文

Soffio 是一套用 Rust 构建的内容发布平台，面向双端体验：公开站点提供静态渲染 + 增量交互的博客浏览，管理站点提供编辑与发布功能。核心由
Axum、Askama、SQLx 构成。
演示站点（整点重置）：
- 公开站点：<https://soffio.xfyyzy.xyz>
- 管理站点：<https://admin.soffio.xfyyzy.xyz>

> 本仓库采用 2-clause BSD 许可（参见 `LICENSE`）。


## 架构速览

```
src/
├── domain        # 领域模型与业务不变量
├── application   # 用例服务、仓库接口、作业调度
├── infra         # Postgres 仓库、HTTP 适配层、缓存、遥测
├── presentation  # 视图模型、模板与布局
├── util          # 附属工具（时区、标识符等）
└── main.rs       # CLI/服务入口
```

核心约束详见 `AGENTS.md`：领域层保持纯函数，副作用隔离在基础设施层。

## 快速开始

1. **准备依赖**
    - Rust 稳定版 >= 1.91（支持 2024 Edition）
    - PostgreSQL 18（默认连接 `postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`）
    - TypeScript Compiler - Version 5.9.3

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
- **缓存**：响应缓存由 `src/infra/cache.rs` 提供，预热器在 `src/infra/cache_warmer.rs`。
- **日志与追踪**：`tracing`, `tracing-subscriber`，统一入口 `src/infra/telemetry.rs`。

## 无头 API

- 基础路径：公共监听上的 `/api/v1`。
- 认证：`Authorization: Bearer <api_key>`，密钥仅在管理后台的“API keys”页面展示一次，操作指南见 [`docs/admin/api-keys.md`](docs/admin/api-keys.md)。
- 权限：通过 scope 控制（蛇形命名）：`post_read`, `post_write`, `page_read`, `page_write`, `tag_read`, `tag_write`, `navigation_read`, `navigation_write`, `upload_read`, `upload_write`, `settings_read`, `settings_write`, `job_read`, `audit_read`。
- 限流：独立配置 `api_rate_limit`（默认 60 秒内每密钥 120 次）。
- 规范：参见 [`docs/api/openapi.yaml`](docs/api/openapi.yaml)。

## 开发工作流

1. 运行格式化与静态检查：
   ```bash
   # 请配置可写数据库；SQLX_TEST_DATABASE_URL 供 `#[sqlx::test]` 创建临时库
   export DATABASE_URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev
   export SQLX_TEST_DATABASE_URL=postgres://soffio:soffio_local_dev@localhost:5432/postgres

   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```
2. 参考 `CONTRIBUTING.md` 获取分支策略与提交要求。
3. 提交 PR 时遵循 `.github/PULL_REQUEST_TEMPLATE.md`，并保持 CI 绿色。

## 部署

生产环境部署指南：

- **Docker 部署**：参见 [`docs/deploy/docker.md`](docs/deploy/docker.md)

## 发布说明

发布记录维护在 `CHANGELOG.md`。每个 Release 请附带：

1. 迁移脚本与数据兼容性说明
2. 新增配置项及默认值变更

## 支持与安全

- 常见问题与支持渠道：`SUPPORT.md`
- 漏洞披露流程：`SECURITY.md`
- 社区行为准则：`CODE_OF_CONDUCT.md`

## 许可证

`BSD-2-Clause` — 详情参见根目录下的 `LICENSE`。
