# Contributing to Soffio

欢迎加入 Soffio！为了保持代码库的可靠性与可审计性，请遵循以下规则。

## 行为准则

所有贡献者必须遵守 `CODE_OF_CONDUCT.md`。如遇到不当行为，请参考其中的报告流程。

## 开发环境要求

- Rust 稳定版 ≥ 1.91（2024 Edition）
- PostgreSQL 18（本地开发可使用仓库附带的迁移）
- TypeScript Compiler - Version 5.9.3
- 可选工具：`sqlx-cli`



## 工作流

1. **分支策略**：从 `main` 派生特性分支，命名建议使用 `feature/<topic>` 或 `fix/<topic>`。
2. **保持颗粒度**：每个 PR 专注单一改动，优先选择最小可行补丁。
3. **编码约束**：
    - 不破坏核心不变量，领域层保持纯逻辑。
    - 避免引入 `unsafe`；如必须使用，请提交安全性论证与测试。
    - 新增依赖须解释必要性，并启用最少特性。
4. **检测流程**（全部通过后再提交 PR）：
   ```bash
   cargo fmt --all -- --check
   cargo check --workspace --all-targets
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets -- --nocapture
   ```
5. **提交信息**：遵循 `AGENTS.md` 的模板（如 `feat(scope): summary`），在正文中说明不变量、边界层级与测试情况。
6. **代码审查**：
    - 在 PR 中链接相关 Issue/讨论。
    - 补充必要的架构或迁移说明。
    - 对公共 API 变更，附上迁移指南和示例。

## 文档与示例

- README、CHANGELOG、docs 目录需随功能更新。

## 发布流程（维护者）

1. 更新 `CHANGELOG.md` 并打上版本标签。
2. 运行完整测试矩阵与数据库迁移。
3. 创建 GitHub Release，附带迁移与兼容性说明。

感谢你的贡献！如有疑问，请在 Issue 或 Discussions 中提出。
