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

Fall back to plain `cargo`/POSIX tools only if necessary.

---

## 2) Repository Boundaries & Layout

Adopt/assume the following layers; **do not** cross them implicitly:

- **core/domain/model** — pure logic, types, state machines. No I/O, no time, no global mutable state.
- **adapters/infra/bin** — I/O, networking, storage, OS bindings, runtime concerns.
- **ffi/sys/unsafe_…** — the only place where `unsafe`/FFI is allowed; small and heavily documented.

Rules:

- Keep invariants inside **core**; expose them via small, versioned interfaces.
- Side effects stay in **adapters**. Inject capabilities explicitly; no hidden singletons.
- All `unsafe` resides in **ffi**; publish safe wrappers with documented preconditions.

---

## 3) Operating Loop (plan → change → verify → deliver)

### 3.1 Plan (Context first, minimal scope)

- Locate targets with zero‑cost discovery
- Preview before editing
- Draft a **Minimal Change Plan** : files, functions, invariants, tests to add/adjust.
- If ambiguity blocks correctness, ask precise questions; otherwise proceed conservatively and record assumptions.

### 3.2 Change (Small, explicit, reversible)

- Keep diffs tight.
- Adding deps:  
  `cargo add <crate>`
  `cargo upgrade --incompatible` (ensure latest version)
  Do **not** raise MSRV; keep `rust-version` pinned in `Cargo.toml` unless explicitly approved.

### 3.3 Verify (gate everything; stop on first failure)

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

### 3.4 Deliver (clear, auditable)

Use a single atomic commit when possible. Use the template in §8.

---

## 4) Idiomatic Rust Rules (enforced)

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

## 5) Dependency Policy (security, minimalism, MSRV)

- Prefer the standard library and existing utilities before adding a new crate.
- **Add** deps only when necessary; with minimal features;`cargo upgrade --incompatible` (ensure latest version)
- **Remove** unused deps with `cargo +nightly udeps`.
- **Update** consciously (`cargo outdated -wR`); avoid MSRV bumps unless explicitly approved. Keep `rust-version` set in
  `Cargo.toml`.

---

## 6) Observability (structured, minimal, safe)

- Emit structured logs at **edge** boundaries; do not log secrets or PII.
- On every new error path, include: `op`, `target`, `correlation_id`, `elapsed_ms`, `result`, `error_code?`, `hint?`.
- Preserve error chains; prefer context‑rich errors over free‑form strings.
- Tests may assert on structured logs where behavior is critical.

---

## 7) Templates (commit)

**Commit message**

```
feat|fix|refactor(scope): imperative summary

Invariants: <list or “unchanged”>
Boundaries: <core/adapters/ffi files touched and why>
Assumptions: <conservative assumptions if any>
Tests: <added/updated tests; feature matrix?>
```

---

## 8) Clarifications, Escalation & Stop Rules

- Ask when necessary for correctness . Provide: the proposed minimal diff, the invariant at
  risk, and the decision needed.
- If any immediate stop condition in §0 triggers, stop and escalate.

---

**End of AGENTS.md**
