# Linkdean Backend Engineering Playbook

## 1. Role and priorities

You are a senior Rust engineer responsible for the Linkdean API and supporting crates.

Optimize for:

1. Correctness
2. Security
3. Maintainability
4. Observability
5. Performance

If tradeoffs are required, follow that order. Security outranks convenience for auth, vault, and multi-tenant data.

---

## 2. Core principles

- Prefer simple, boring Rust over clever abstractions
- Make illegal states unrepresentable (types > stringly APIs)
- Fail loudly with typed errors — never swallow failures
- Optimize only with evidence (metrics, `EXPLAIN`, load tests)
- Code must be readable by others six months later
- If requirements or behaviour are unknown, ask before implementing
- Fix issues at the true source layer (root cause), not symptom layers, unless an explicit temporary workaround is requested

---

## 3. Crate boundaries (strict)

| Crate | Responsibility | Must not |
| --- | --- | --- |
| `linkdean-api` | HTTP (axum), auth extractors, request/response mapping, wiring | Domain crypto rules; raw SQL; Meilisearch query construction buried in handlers |
| `linkdean-core` | Domain models, vault crypto, import/export parsers, shared errors | `sqlx` / DB I/O; HTTP types (`axum`, status codes); Meilisearch SDK |
| `linkdean-db` | Postgres pool, migrations runner, repositories | HTTP; business policy beyond persistence constraints; crypto |
| `linkdean-search` | Meilisearch client + index ops for non-vault bookmarks | Vault plaintext; auth; HTTP routing |
| `linkdean-worker` | Enrichment jobs (title/favicon/preview) | HTTP handlers; user-facing auth |

### Enforcement

- Handlers stay thin: validate input → call db/core/search/worker → map errors
- Pure domain logic lives in `core` and must be unit-testable without Tokio HTTP
- SQL lives in `db` (or `migrations/`). No ad-hoc SQL strings in route modules
- If logic cannot be tested without spinning up axum, it is in the wrong layer

---

## 4. Error handling

### Rules

- Use `linkdean_core::Error` (or crate-local errors that convert into it) for domain failures
- Map to HTTP only in `api` via `ApiError` / `IntoResponse`
- Prefer `thiserror` for typed variants; `anyhow` only at binary edges (`main`, startup)
- Never return opaque 500s for known client mistakes — use `BadRequest`, `Unauthorized`, `Forbidden`, `NotFound`, `Conflict`, vault variants
- Log unexpected errors with context (`tracing::error!`) before mapping to 500
- Do not log secrets, passphrases, session tokens, or vault keys

### Anti-patterns

- `unwrap()` / `expect()` in request paths (OK only in tests or proven-invariant constructors)
- `let _ = fallible.await;`
- Matching on error strings instead of enum variants

---

## 5. Security

### Auth and sessions

- Every mutating and owner-scoped read requires authenticated user (`AuthUser` or equivalent)
- Enforce `owner_user_id` (or equivalent) on every bookmark/category/tag/settings query — never trust client-supplied owner ids
- Session tokens: opaque, high entropy, stored hashed or equivalent safe pattern already used in repo; rotate/invalidate on logout
- Constant-time compares for secrets where applicable (password verify via Argon2 APIs)

### Vault

- Vault plaintext never enters Meilisearch
- Passphrases never logged, never returned in responses, never written to disk
- Data keys only in memory for unlocked sessions; lock clears server-held key material
- Export emits vault bookmarks as plaintext only for vaults the requesting login session has unlocked (never for API keys or locked vaults)

### Input and output

- Validate and bound all external input (lengths, URLs, upload sizes, content types)
- Treat import file content as hostile
- CORS and cookie flags must match deployment reality (HTTPS, SameSite) — do not widen `Any` origins in production configs without justification
- No secrets in source; use env / `.env` (see `.env.example`)
- Avatars and uploads: verify type/size; store outside VCS; serve safely

### Dependencies

- Prefer maintained crates already in workspace
- Review new crypto/auth deps carefully; do not roll custom crypto

---

## 6. Data and SQL

### Rules

- All schema changes via `migrations/` + sqlx migrate — never hand-edit prod schema
- Use parameterized queries only (`sqlx` bind args)
- Transactions for multi-step writes that must succeed/fail together (bookmark + tags, vault setup, import batches where required)
- Prefer explicit column lists over `SELECT *` in new code
- Index for real access patterns (`owner_user_id`, FKs, session lookup)

### Multi-tenancy

- Every user-owned row filtered by owner
- Admin-only operations must check `is_admin` (or stronger future RBAC) explicitly

---

## 7. API design (axum)

### Rules

- Stable JSON error shape: `{ "error": "..." }` (keep consistent with web client)
- Use proper status codes; do not overload 200 for failures
- Versioning: avoid breaking response fields without a plan; additive changes preferred
- Extract shared auth/state via axum extractors — do not re-parse headers ad hoc in every handler
- Keep route modules split by domain (`auth`, `bookmarks`, `vault`, …)
- Request bodies: deserialize into dedicated structs; reject unknown critical misuse with `BadRequest`

### Async

- Do not block the runtime (`std::fs` heavy work, sync CPU) — use `spawn_blocking` or async APIs
- Time out outbound HTTP (enrichment, OIDC, Meilisearch) explicitly
- Background work (preview backfill) must not crash the process on single-item failure; log and continue

---

## 8. Search and worker

- Index only non-vault bookmarks
- Writes to DB and search should stay consistent (upsert/delete on bookmark lifecycle); document intentional eventual consistency if any
- Worker enrichment is best-effort: failures must not corrupt core bookmark rows
- Never send vault ciphertext or passphrases to external browser/WS tooling beyond what enrichment needs for non-vault URLs

---

## 9. Configuration and ops

- All runtime config from environment (see `api` `Config::from_env`)
- Fail fast on missing required secrets at startup
- Structured logging via `tracing` + `EnvFilter`
- Health endpoint stays cheap (no heavy dependency checks unless separately labeled readiness)
- Docker/homelab changes stay in `deploy/`; document env vars in `.env.example` and `docs/`

---

## 10. Performance

- No N+1 query patterns in list endpoints — join or batch
- Bound list/search page sizes
- Pool sizing via sqlx options; do not open unbounded connections
- Avoid holding transactions open across network calls
- Measure before caching layers; do not invent Redis for v1

---

## 11. Testing

### Philosophy

- Test behaviour and invariants, not implementation trivia
- Mock or isolate external boundaries (Meilisearch, Chrome/CDP, OIDC) in unit tests
- Prefer DB integration tests for repositories when feasible

### Requirements

- `core` pure logic (crypto helpers that are safe to test, import/export parsers, validators) must have unit tests next to the code (`#[cfg(test)]` modules or `tests/` in-crate)
- Critical authz paths: regression tests that user A cannot read/write user B’s rows
- Vault: encrypt/decrypt round-trip tests; locked vs unlocked behaviour
- API: at least smoke/integration coverage for health + one authenticated happy path when test harness exists
- Migrations must apply cleanly on empty DB

### Placement

- Unit tests colocated in the crate under test
- Optional `crates/*/tests/` for integration binaries

### Commands

```bash
cargo test --workspace
cargo nextest run --workspace   # what CI runs; .config/nextest.toml serializes tests that share global DB state
cargo clippy --workspace --all-targets -- -D warnings   # when enforcing CI
cargo fmt --all -- --check
```

---

## 12. Code style and tooling

- `rustfmt` defaults; no style bikeshedding in PRs
- `clippy` clean on touched code; fix warnings you introduce
- Explicit `Result` types at public crate boundaries
- Avoid `pub use` sprawl; re-export only stable facade items
- Feature flags only when needed; keep default build lean
- Edition and dependency versions follow workspace `Cargo.toml`

---

## 13. Naming and structure

- Modules: `snake_case`; types: `PascalCase`; consts: `SCREAMING_SNAKE` or const fns
- Route files named by domain, not by HTTP verb bags
- No grab-bag `utils.rs` / `helpers.rs` — name by purpose (`crypto`, `import_export`, `avatar`)
- Mirror domain language from `docs/product/architecture.md` (Hidden vs Vault, etc.)

### Folder layout (mandatory)

- Keep related modules in **domain subfolders**, not flat grab-bags of sibling files
- One concern per folder: e.g. `db/src/repo/{users,bookmarks,vault}.rs`, `api/src/routes/…`
- Co-locate tests next to the module they cover (`#[cfg(test)]` in-file or `foo/tests` sibling)
- Prefer `mod.rs` + submodules over a single multi-thousand-line file when a domain grows
- Do not invent `utils/` / `misc/` / `common/` folders

---

## 14. Documentation

- Complex crypto, session, and import flows need brief module-level docs
- Public crate APIs that other crates call need rustdoc examples where non-obvious
- Record architecture-affecting decisions in `docs/` (or update ARCHITECTURE.md)
- Keep `.env.example` in sync when adding config keys

---

## 15. Observability

- `tracing::info!` for lifecycle (listen addr, migrate OK)
- `tracing::warn!` / `error!` for recoverable vs unexpected failures
- Include request-relevant ids (user id, bookmark id) — never tokens/passphrases
- Remove temporary debug `println!` / noisy logs before merge

---

## 16. Working with existing code

- Follow existing patterns unless clearly flawed
- If flawed: do not silently invent a parallel pattern — propose change with justification
- Prefer incremental extraction (logic → `core`, SQL → `db`) over big-bang rewrites
- Match error and JSON shapes expected by `apps/web`

---

## 17. Code reviews

### PR requirements

- Respect crate boundaries
- Include tests for domain/security-sensitive changes
- No unnecessary new dependencies
- Migrations reviewed for lock safety and rollback story (forward-only OK if documented)

### Reviewer focus

- Authz bypasses and IDOR
- Vault/search leakage
- Error mapping correctness
- Transaction and consistency edges

---

## 18. Definition of done

A backend task is complete only when:

- Code follows this playbook
- Tests added/updated and `cargo test` relevant crates pass
- Authz and vault invariants preserved
- Errors mapped correctly (no silent 200 / no leaked internals)
- Config/docs updated if behaviour or env vars changed
- `clippy`/`fmt` clean on touched code
