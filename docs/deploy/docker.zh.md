# Docker 部署

[English](docker.md) | 中文

本文档说明如何使用仓库内的 `deploy/docker/Dockerfile` 构建并运行 Soffio 的生产镜像，同时给出运行时配置、健康检查与常见运维命令。

## 镜像结构

- **Builder**：基于 `rust:1.91-alpine3.20`，集成 `cargo-chef` 与 TypeScript 编译器，通过 `TARGET_TRIPLE`（默认
  `x86_64-unknown-linux-musl`）与 `TARGET_CPU`（默认 `x86-64-v2`）编译静态二进制。
- **Runtime**：继承 `MERMAID_CLI_IMAGE`（默认 `minlag/mermaid-cli:latest`），同时注入 `soffio` 二进制与 Mermaid CLI
  包装脚本，提供服务端图表渲染能力。


## 构建镜像

```bash
docker buildx build \
  --platform linux/amd64 \
  --build-arg TARGET_TRIPLE=x86_64-unknown-linux-musl \
  --build-arg TARGET_CPU=x86-64-v2 \
  --build-arg MERMAID_CLI_IMAGE=minlag/mermaid-cli:10.5.1 \
  -f deploy/docker/Dockerfile \
  -t soffio:latest \
  .
```



## 容器运行配置

运行时所有配置均来自环境变量（参见 `soffio.toml.example`）。常用项如下：

| 变量                                                    | 说明            | 默认值                                               |
|-------------------------------------------------------|---------------|---------------------------------------------------|
| `SOFFIO__DATABASE__URL`                               | Postgres 连接串  | 必填，示例 `postgres://soffio:***@db:5432/soffio_prod` |
| `SOFFIO__SERVER__PUBLIC_PORT`                         | 公共站点监听端口      | `3000`                                            |
| `SOFFIO__SERVER__ADMIN_PORT`                          | 管理端监听端口       | `3001`                                            |
| `SOFFIO__SERVER__HOST` / `SOFFIO__SERVER__ADMIN_HOST` | 监听地址          | 构建镜像时默认 `0.0.0.0`                                 |
| `SOFFIO__UPLOADS__DIRECTORY`                          | 上传文件持久化目录     | `/var/lib/soffio/uploads`                         |
| `SOFFIO__LOGGING__JSON`                               | 输出 JSON 结构化日志 | `false`                                           |

Soffio 在启动时会自动执行数据库迁移（`PostgresRepositories::run_migrations`），因此请确保数据库用户具备创建表的权限。

## 运行示例

```bash
docker run -d \
  --name soffio \
  -p 3000:3000 \
  -p 3001:3001 \
  -v soffio_uploads:/var/lib/soffio/uploads \
  -e RUST_LOG=info \
  -e SOFFIO__DATABASE__URL=postgres://soffio:soffio_prod@postgres:5432/soffio_prod \
  -e SOFFIO__SERVER__HOST=0.0.0.0 \
  -e SOFFIO__SERVER__ADMIN_HOST=0.0.0.0 \
  ghcr.io/xfyyzy/soffio:amd64-x86-64-v2
```

健康检查端点：

- 公共站点：`GET /_health/db`
- 管理站点：`GET /_health/db`（监听在 `SOFFIO__SERVER__ADMIN_PORT`）

## 使用 docker compose

仓库根目录提供 `docker-compose.yml`，涵盖 PostgreSQL 与 Soffio 应用容器：

```bash
docker compose -f docker-compose.yml up -d
```

部署前请根据环境修改：

- 将 `SOFFIO__DATABASE__URL` 指向生产数据库。
- 调整 `volumes` 将 `/var/lib/soffio/uploads` 挂载到生产存储（默认示例使用 `./update`）。
- 如需开启管理端，也可映射 `3001` 端口或使用内网访问。
