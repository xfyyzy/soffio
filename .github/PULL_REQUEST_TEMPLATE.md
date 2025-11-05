# PR Checklist

感谢你的贡献！请确认以下事项：

## 类型
- [ ] Bug 修复
- [ ] 新功能
- [ ] 重构 / 清理
- [ ] 文档
- [ ] 其他（请说明）：<!-- 填写 -->

## 描述
<!-- 简要说明改动目的、背景以及对用户/系统的影响。 -->

## 验证
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets -- --nocapture`
- [ ] 其他验证（请列出）：<!-- 可选 -->

## 风险与迁移
- 相关配置/环境变量变更：
- 数据库迁移：
- 受影响的缓存或后台任务：

## 关联
- Issue / 讨论链接：
- 文档更新：<!-- 如无请写 N/A -->
