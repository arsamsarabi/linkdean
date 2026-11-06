# Structured logs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship roadmap #9 — JSON/text logging, optional JSONL file sink, and `X-Request-Id` on every HTTP response correlated into request spans.

**Architecture:** Keep `tracing` + `TraceLayer`. Add a small `request_id` normalize step (strip invalid inbound ids), then `tower_http` Set/Propagate request-id layers, with `MakeSpan` recording `request_id`. Replace single `fmt().init()` with a `Registry` of stdout (text|json) plus optional always-JSON file layer.

**Tech Stack:** Rust, axum, tower-http `request-id`, tracing / tracing-subscriber (`env-filter`, `json`), uuid v4.

**Spec:** `docs/superpowers/specs/2026-11-06-structured-logs-design.md`

---

### Task 1: Workspace features

**Files:**
- Modify: `Cargo.toml` (workspace deps)

- [ ] **Step 1:** Add features:

```toml
tower-http = { version = "0.6", features = ["cors", "trace", "fs", "request-id"] }
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

- [ ] **Step 2:** `cargo check -p linkdean-api` succeeds (may pull crates).

- [ ] **Step 3:** Commit `chore: enable tracing json and tower-http request-id`

---

### Task 2: Request ID validation + HTTP layers (TDD)

**Files:**
- Create: `crates/api/src/request_id.rs`
- Modify: `crates/api/src/main.rs` (mod + layers)
- Modify: `crates/api/src/lib` — N/A (binary crate; tests in module)

- [ ] **Step 1: Failing tests** in `request_id.rs` for `is_valid_request_id` / `normalize_header_value`:

```rust
#[test]
fn accepts_uuid_and_simple_tokens() {
    assert!(is_valid_request_id("550e8400-e29b-41d4-a716-446655440000"));
    assert!(is_valid_request_id("cf-ray-abc_123.def"));
}

#[test]
fn rejects_empty_spaces_and_overlong() {
    assert!(!is_valid_request_id(""));
    assert!(!is_valid_request_id("bad id"));
    assert!(!is_valid_request_id(&"a".repeat(129)));
}

#[tokio::test]
async fn response_echoes_valid_inbound_id() { /* Router + normalize + Set + Propagate; assert header */ }

#[tokio::test]
async fn response_mints_when_missing_or_invalid() { /* … */ }
```

- [ ] **Step 2:** Run tests — expect compile/fail until impl exists.

- [ ] **Step 3: Implement** `is_valid_request_id` (1–128 chars, `A–Za–z0–9_-.`), `normalize_request_id` middleware (remove invalid `x-request-id`), `MakeRequestUuid` implementing `MakeRequestId`, `http_trace_layer()` returning TraceLayer with `request_id` span field from `RequestId` extension, and `request_id_layers` helpers or wire in `main`.

Layer order on the outer app router (last `.layer` = outermost):

1. existing cors, security (unchanged relative order among themselves)
2. `PropagateRequestIdLayer::x_request_id()`
3. `TraceLayer` with make_span
4. `SetRequestIdLayer::x_request_id(MakeRequestUuid)`
5. `normalize_request_id` middleware (outermost of the request-id stack)

- [ ] **Step 4:** Tests pass: `cargo test -p linkdean-api -j 2 request_id -- --nocapture`

- [ ] **Step 5:** Commit `feat(api): X-Request-Id header with validated inbound ids`

---

### Task 3: Logging subscriber (TDD where cheap)

**Files:**
- Create: `crates/api/src/logging.rs`
- Modify: `crates/api/src/main.rs` (call `logging::init()?` instead of inline fmt)

- [ ] **Step 1: Tests** for `LogFormat::parse` (`text`/`json`/default/invalid) and optionally writer behaviour if easy; fail-fast message for bad `LOG_FORMAT`.

- [ ] **Step 2:** Implement `logging::init() -> anyhow::Result<()>`:
  - `EnvFilter` as today
  - stdout layer text or json from `LOG_FORMAT` (default `text`)
  - if `LOG_FILE` set non-empty: append JSON layer to that path; open errors → `Err`
  - `registry.with(filter).with(stdout).with(file_layer).init()`

- [ ] **Step 3:** `cargo test -p linkdean-api -j 2 logging` and smoke `cargo check -p linkdean-api`

- [ ] **Step 4:** Commit `feat(api): LOG_FORMAT and optional JSONL LOG_FILE`

---

### Task 4: Docs + close roadmap #9

**Files:**
- Modify: `docs/INSTALL.md` (Logging subsection; `LOG_FORMAT`, `LOG_FILE`; leave `RUST_LOG`)
- Modify: `.env.example` (commented `LOG_FORMAT` / `LOG_FILE`)
- Modify: `docs/ARCHITECTURE.md` (observability bullet near metrics)
- Modify: `docs/FEATURES.md` (Ops & self-host bullet)
- Modify: `docs/ROADMAP.md` (mark #9 Done like #7/#8)
- Modify: `docs/MANAGED_SERVICE.md` (next pointer past #9 → marketing)

- [ ] **Step 1:** Apply doc edits matching design success criteria.

- [ ] **Step 2:** Commit `docs: ship structured logs as roadmap #9`

- [ ] **Step 3:** `graphify update .` ; push branch; give compare URL (gh pr create may fail as arsamoni).

---

### Spec coverage check

| Spec item | Task |
| --- | --- |
| LOG_FORMAT text/json | 3 |
| LOG_FILE always JSONL | 3 |
| X-Request-Id response | 2 |
| Prefer valid inbound | 2 |
| Span request_id | 2 |
| Docs + roadmap close | 4 |
| No rotation / no OTel | out of scope |
