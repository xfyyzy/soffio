# AGENTS.md

> **Objective:** ship **minimal, safe, auditable** Rust changes that preserve invariants and align with this codebase’s
> architecture philosophy.  
> **Scope precedence (within this repository):** this file’s rules **override ad‑hoc task instructions** when they
> conflict. When unsure, **fail closed** and ask up to 3 concise questions.

---

## 0) Agent Contract (non‑negotiables)

You MUST:

1. **Prioritize correctness over speed.** Encode invariants in types; prefer compile‑time checks over runtime
   heuristics.
2. **Minimize the Trusted Computing Base.** Touch the smallest set of files; keep `unsafe`/FFI in dedicated modules;
   never widen privileges implicitly.
3. **Be explicit and auditable.** Every sensitive change MUST be testable, observable, and explainable via diffs, logs,
   and metrics.
4. **Keep work deterministic and reproducible.** Same inputs ⇒ same outputs. Do not change toolchain or dependency
   constraints casually.
5. **Constrain change to the boundary.** Keep domain/core stable; place side effects in adapters/edges unless explicitly
   requested otherwise.
6. **Fail closed.** If correctness is uncertain, stop and request clarification . If silence
   persists, implement the **conservative** option and document assumptions.

**Immediate stop conditions (ask before proceeding):**

- Breaking an invariant or public API without migration.
- Introducing or expanding `unsafe`/FFI without a safety proof and tests.
- Too many changes in a single atomic commit (split or justify).
- Elevating privileges, adding global mutable state, or hiding behavior behind undocumented env flags.

---

## 1) Preferred Tooling (preinstalled; use these first)

- **Search & navigation:** `rg`, `fd`
- **Viewing & editing aids:** `bat`, `sd`
- **Cargo helpers:** `cargo-edit` (`cargo add`/`rm`/`set-version`), `cargo-hack`, `cargo-watch`, `cargo-expand`,
  `cargo-udeps`, `cargo-deny`, `cargo-audit`, `cargo-outdated`
- **VCS & GitHub:** `git` and GitHub access are already configured and the `gh` CLI is installed. Use `gh` for
  GitHub operations only when explicitly requested by the user; `git` commands remain governed by existing git rules.

Fall back to plain `cargo`/POSIX tools only if necessary.

---

## 2) Development Environment

**Database**

Development database is managed by Docker Compose:

```bash
docker compose -f docker-compose-dev.yml up -d
```

Connection URL: `postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`

**SQLx**

Prefer compile-time checked macros (`sqlx::query!`, `sqlx::query_as!`, `sqlx::query_scalar!`) over runtime functions (`sqlx::query`, `sqlx::query_as`). For complex dynamic queries, `QueryBuilder` is acceptable.

After modifying queries, regenerate compile-time checked query metadata:

```bash
cargo sqlx prepare --workspace --database-url postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev -- --all-targets
```

**Static assets**

Static assets are embedded into the binary even in development; restart the service after changing assets (CSS, templates, etc.) to see the effect.

---

## 3) Repository Boundaries & Layout

Adopt/assume the following layers; **do not** cross them implicitly:

- **core/domain/model** — pure logic, types, state machines. No I/O, no time, no global mutable state.
- **adapters/infra/bin** — I/O, networking, storage, OS bindings, runtime concerns.
- **ffi/sys/unsafe_…** — the only place where `unsafe`/FFI is allowed; small and heavily documented.

Rules:

- Keep invariants inside **core**; expose them via small, versioned interfaces.
- Side effects stay in **adapters**. Inject capabilities explicitly; no hidden singletons.
- All `unsafe` resides in **ffi**; publish safe wrappers with documented preconditions.

---

## 4) Operating Loop (plan → change → verify → deliver)

### 4.1 Plan (Context first, minimal scope)

- **Pattern reuse first:** Before designing a solution, search the codebase for existing patterns solving similar problems. Ensure consistent solutions for identical problems. Always prefer reusing shared code over duplicating logic. If shared code doesn't fit your need, extend it rather than reimplementing.
- **Research when uncertain:** When facing complex problems or lacking information, proactively use web search to find solutions or verify uncertain details.
- **Confirm before executing:** Always present your proposed approach first and wait for user confirmation before implementation.
- Locate targets with zero‑cost discovery
- Preview before editing
- Draft a **Minimal Change Plan** : files, functions, invariants, tests to add/adjust.
- If ambiguity blocks correctness, ask precise questions; otherwise proceed conservatively and record assumptions.

### 4.2 Change (Small, explicit, reversible)

- Keep diffs tight.
- Adding deps:  
  `cargo add <crate>`
  `cargo upgrade --incompatible` (ensure latest version)
  Do **not** raise MSRV; keep `rust-version` pinned in `Cargo.toml` unless explicitly approved.

### 4.3 Verify (gate everything; stop on first failure)

run the sequence below:

1) **Format** — `cargo fmt --all -- --check`
2) **Build & Clippy** —  
   `cargo check --workspace --all-targets`  
   `cargo clippy --workspace --all-targets -- -D warnings`
3) **Tests** — `cargo test --workspace --all-targets -- --nocapture`  
   If features exist: `cargo hack test --workspace --feature-powerset --depth 1`
4) **Deps & Risk** —  
   `cargo +nightly udeps --all-targets --workspace` (unused deps)
   `cargo outdated -wR` (report only; do not upgrade unless asked)
5) **Macros (if touched)** — `cargo expand -p <crate> --lib` and sanity‑scan the output.

### 4.4 Deliver (clear, auditable)

Use a single atomic commit when possible. Use the template in §9.

- **Do not use `git commit --amend`** unless the user explicitly requests it.

### 4.5 Changelog & Release discipline

- Always identify and label Breaking changes in `CHANGELOG.md`.
- Default behavior: log every codebase-related change under **Unreleased** (in the same commit as the code change or in a dedicated changelog commit).
- Release flow (run **only when the user explicitly asks to publish**):
  1) Bump version in `Cargo.toml` to the user-specified value.
  2) Run the full gate (fmt, clippy, tests, etc.) with `SQLX_TEST_DATABASE_URL=postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres DATABASE_URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`; accept snapshot diffs caused by the version bump.
  3) Update `CHANGELOG.md`: add the new version section, move Unreleased contents there, and fill in any missing release notes.
  4) After user reconfirms, create the release via `gh`: tag `vX.Y.Z`, title `vX.Y.Z - <brief title>`, release notes based on that version’s changelog entry.

---

## 5) Idiomatic Rust Rules (enforced)

**Language**

- Use English for all code comments, documentation, commit messages, and user-facing text in the codebase.

**Types & invariants**

- Encode invariants in types (enums/newtypes/state machines). Illegal states should be unrepresentable.
- Prefer small, pure functions. No global mutable state.

**Error handling**

- Libraries: precise error types (e.g., `thiserror`); **no** `unwrap`/`expect`/`panic!` except in tests.
- Binaries: may use a top‑level error reporter; propagate context; never swallow errors.

**API design**

- Public items MUST have `rustdoc` and a runnable doctest example.
- Keep semver discipline; provide migration notes for breaking changes.
- Follow the Rust API guidelines (naming, visibility, trait bounds, feature flags, forward compatibility).

**Concurrency & async**

- Prefer message‑passing and immutable sharing.
- **Never hold a lock across `.await`.**
- **No blocking I/O** in async contexts (use adapters or `spawn_blocking`).
- Document `Send`/`Sync` expectations when crossing threads/runtimes.
- **Async job payloads:** Job payloads should carry complete execution context. Workers should not re-read mutable data that was available at enqueue time; this prevents race conditions when separate connection pools are used for HTTP requests and job workers. Exception: scheduled/delayed jobs that intentionally need the latest state at execution time.

**Unsafe & FFI**

- Only in `ffi/sys/unsafe_…` modules.
- Every change MUST include: a safety comment (preconditions, aliasing/lifetime guarantees, panic behavior), unit tests,
  and—if concurrency is involved—rationale for absence of UB/data races.

**Clippy baseline**

- Always run with `-D warnings` and justify any `allow` inline.

**Testing**

- Unit + integration tests around invariants and error paths.
- Property tests for state machines where feasible.
- Use textual golden files only when stable and reviewed.

---

## 6) Dependency Policy (security, minimalism, MSRV)

- Prefer the standard library and existing utilities before adding a new crate.
- **Add** deps only when necessary; with minimal features;`cargo upgrade --incompatible` (ensure latest version)
- **Remove** unused deps with `cargo +nightly udeps`.
- **Update** consciously (`cargo outdated -wR`); avoid MSRV bumps unless explicitly approved. Keep `rust-version` set in
  `Cargo.toml`.

---

## 7) Observability (structured, minimal, safe)

- Emit structured logs at **edge** boundaries; do not log secrets or PII.
- On every new error path, include: `op`, `target`, `correlation_id`, `elapsed_ms`, `result`, `error_code?`, `hint?`.
- Preserve error chains; prefer context‑rich errors over free‑form strings.
- Tests may assert on structured logs where behavior is critical.

---

## 8) Frontend Interaction & Styling

- **Interaction:** Keep zero JavaScript; use the existing datastar + SSE infrastructure for fully server‑driven flows. Reuse the provided patterns instead of introducing client-side JS.
- **Selectors & styling:** Maintain the zero‑class principle; use custom tags and `data-*` attributes as selectors. When adding styles, align with the existing visual language.

---

## 9) Templates (commit)

**Commit message**

```
feat|fix|refactor(scope): imperative summary

Invariants: <list or “unchanged”>
Boundaries: <core/adapters/ffi files touched and why>
Assumptions: <conservative assumptions if any>
Tests: <added/updated tests; feature matrix?>
```

---

## 10) Clarifications, Escalation & Stop Rules

- Ask when necessary for correctness . Provide: the proposed minimal diff, the invariant at
  risk, and the decision needed.
- If any immediate stop condition in §0 triggers, stop and escalate.

---

**End of AGENTS.md**
