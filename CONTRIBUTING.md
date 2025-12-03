# Contributing to Soffio

English | [中文](CONTRIBUTING.zh.md)

Welcome! To keep the codebase reliable and auditable, please follow the guidelines below.

## Code of Conduct

All contributors must follow `CODE_OF_CONDUCT.md`. If you encounter improper behavior, use the reporting process described there.

## Development Requirements

- Rust stable ≥ 1.91 (2024 Edition)
- PostgreSQL 18 (local dev can rely on the bundled migrations)
- TypeScript Compiler 5.9.3
- Optional tooling: `sqlx-cli`

## Workflow

1. **Branching**: Branch from `main`. Prefer names like `feature/<topic>` or `fix/<topic>`.
2. **Scope**: Keep each PR focused on a single change and aim for the smallest viable patch.
3. **Coding constraints**:
   - Preserve core invariants; keep the domain layer pure.
   - Avoid introducing `unsafe`. If unavoidable, include a safety rationale and tests.
   - Justify new dependencies and enable the minimal feature set.
4. **Checks** (run all before opening a PR):
   ```bash
   cargo fmt --all -- --check
   cargo check --workspace --all-targets
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets -- --nocapture
   # If you change soffio-cli commands/options, regenerate docs:
   cargo run --bin gen_cli_docs
   ```
5. **Commit messages**: Follow the template in `AGENTS.md` (e.g., `feat(scope): summary`). Document invariants, touched boundaries, and test coverage in the body.
6. **Code review**:
   - Link related Issues/Discussions in the PR.
   - Provide architecture or migration notes when needed.
   - For public API changes, include migration guidance and examples.

## Docs & Examples

- Keep README, CHANGELOG, and files under `docs/` up to date with your changes.

## Release Flow (Maintainers)

1. Update `CHANGELOG.md` and tag the version.
2. Run the full test matrix plus database migrations.
3. Publish a GitHub Release with migration and compatibility notes.

Thanks for contributing! If you have questions, open an Issue or start a Discussion.
