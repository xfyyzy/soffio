# Soffio

[![CI](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml)
[![Release](https://github.com/xfyyzy/soffio/actions/workflows/release.yml/badge.svg)](https://github.com/xfyyzy/soffio/actions/workflows/release.yml)
[![Rustc](https://img.shields.io/badge/rustc-1.91%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/tools/install)

Soffio 是一个用 Rust 构建的内容发布平台：公开站点提供静态渲染 + 增量交互的阅读体验，管理站点面向编辑、排版与发布。核心技术栈为 Axum、Askama 与 SQLx，严格遵循领域/应用/基础设施分层（详见 `AGENTS.md`）。本仓库采用 BSD-2-Clause 许可。

## 演示环境

- 公开站点：<https://soffio.xfyyzy.xyz>
- 管理站点：<https://admin.soffio.xfyyzy.xyz>

（演示数据按整点重置。）

## 仓库布局

```
src/
├── domain        # 领域模型与业务不变量
├── application   # 用例服务、仓库接口、作业调度
├── infra         # Postgres 仓库、HTTP 适配层、缓存、遥测
├── presentation  # 视图模型、模板与布局
├── util          # 附属工具（时区、标识符等）
└── main.rs       # CLI/服务入口
```

## 依赖要求

- Rust 稳定版 ≥ 1.91（支持 2024 Edition）
- PostgreSQL 18（默认连接 `postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`）
- TypeScript Compiler 5.9.3

## 快速开始

1. **安装依赖**：确保以上三项均可用，并创建开发数据库 `soffio_dev`。
2. **启动服务**：
   ```bash
   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev cargo run --bin soffio
   ```
3. **访问端口**：公共站点监听 `127.0.0.1:3000`，管理站点监听 `127.0.0.1:3001`；可通过 CLI 参数或环境变量覆盖。

## 核心运行时组件

- **HTTP 服务**：Axum 8，同进程内区分公开/管理监听；公共路由位于 `src/infra/http/public.rs`，管理路由位于 `src/infra/http/admin/`。
- **数据库访问**：SQLx Postgres，仓库实现集中在 `src/infra/db`，接口定义在 `src/application/repos.rs`。
- **缓存**：响应缓存由 `src/infra/cache.rs` 提供，预热逻辑在 `src/infra/cache_warmer.rs`。
- **遥测**：`tracing` + `tracing-subscriber`，统一初始化入口 `src/infra/telemetry.rs`。

## 开发工作流

1. 运行基础质量门槛：
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```
2. 参考 `CONTRIBUTING.md` 了解分支命名、提交模板与代码审查要求。
3. 提交 PR 时遵循 `.github/PULL_REQUEST_TEMPLATE.md` 并确保 CI 全绿。

## 部署指南

生产部署建议使用 Docker（详见 [`docs/deploy/docker.md`](docs/deploy/docker.md)）。其他部署形态可参考该文档涉及的环境变量、卷挂载与健康检查逻辑。

## 发布与变更记录

- 所有 Release 说明记录在 `CHANGELOG.md`。
- 每次发布务必附带：
  1. 数据迁移脚本与兼容性提示；
  2. 新增/变更配置项及默认值说明。

## 支持、社区与安全

- 常见问题与支持渠道：`SUPPORT.md`
- 漏洞披露流程：`SECURITY.md`
- 行为准则：`CODE_OF_CONDUCT.md`

## 许可证

BSD-2-Clause，详情参见 `LICENSE`。
