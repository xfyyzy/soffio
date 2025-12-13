# PR Checklist

Thank you for your contribution! Please confirm the following:

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Refactor / cleanup
- [ ] Documentation
- [ ] Other (please specify): <!-- fill in -->

## Description
<!-- Briefly describe the purpose, context, and impact on users/system. -->

## Verification
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets -- --nocapture`
- [ ] Other verification (list): <!-- optional -->

## Risk & Migration
- Related config/env changes:
- Database migrations:
- Affected caches or background jobs:

## Links
- Issue / discussion link:
- Documentation updates: <!-- write N/A if none -->
