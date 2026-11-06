# Structured logs (roadmap #9)

Design for request-correlated JSON logging before free public beta. Agreed 2026-11-06.

## Problem

The API already logs with `tracing` and `tower_http::TraceLayer`, but only as human text on stdout. Self-hosters and the future managed service cannot grep or ship logs by request, and there is no `X-Request-Id` to match a browser report to a line.

## Goals

- Opt-in JSON on stdout via `LOG_FORMAT=json` (default stays human text).
- Optional append-only **JSONL** file via `LOG_FILE` (always JSON lines, even when stdout is text).
- Every HTTP response includes `X-Request-Id`.
- That same id appears on log lines for work under that request (span field `request_id`).
- Prefer a valid inbound `X-Request-Id` (proxy correlation); otherwise mint a UUID.
- Document the new env vars in INSTALL / `.env.example`; note observability in ARCHITECTURE; move #9 to FEATURES when shipped.

## Non-goals (this item)

- Log rotation or retention (operators use logrotate / Docker logging drivers).
- OpenTelemetry / distributed tracing exporters (later Backlog item, will reuse `request_id`).
- Changing `RUST_LOG` / `EnvFilter` behaviour.
- Separate worker binary subscriber (worker tasks run inside the API process today).
- Emitting request ids from the web app UI (header is enough for support copy-paste later).

## Approaches considered

| Approach | Pros | Cons |
| --- | --- | --- |
| **A. `tracing-subscriber` layers + `tower_http` request-id + TraceLayer MakeSpan** (chosen) | Matches existing stack; dual stdout/file layers; small code surface | Span nesting must be wired so `request_id` shows on events |
| B. Custom printf JSON middleware only | Simple for access logs | Misses existing `tracing::warn!` / maintenance logs; duplicates TraceLayer |
| C. Full OpenTelemetry now | Future-proof | Out of scope; heavier deps |

## Design

### Config

Read at subscriber init (before or with `Config::from_env`; logging must start early):

| Variable | Default | Behaviour |
| --- | --- | --- |
| `LOG_FORMAT` | `text` | `text` → human fmt on stdout; `json` → JSONL on stdout. Other values → fail fast at startup. |
| `LOG_FILE` | unset | When set, append JSONL to that path (create if missing). Open failure → fail fast. |
| `RUST_LOG` | unchanged | Still drives `EnvFilter` (default `info`). |

`LOG_FILE` format does **not** follow `LOG_FORMAT`: the file is always JSONL.

### Subscriber

Replace the single `tracing_subscriber::fmt().init()` in `crates/api/src/main.rs` with a `Registry` plus:

1. **Stdout layer** — text or JSON per `LOG_FORMAT`.
2. **Optional file layer** — JSON only, `Mutex`/`parking_lot` around the append `File` writer (or `tracing_appender` non-blocking only if we already depend on it; prefer std `Mutex` + file for YAGNI).

Enable workspace `tracing-subscriber` feature `json`.

### Request ID

- Enable `tower-http` feature `request-id`.
- `SetRequestIdLayer` with a custom `MakeRequestId` that:
  - If the request already has `X-Request-Id`, accept it only when it matches a tight allowlist (e.g. 1–128 chars, `A–Z a–z 0–9 _ - .`); otherwise mint `Uuid::new_v4()` as a hyphenated string.
  - If absent, mint a UUID.
- `PropagateRequestIdLayer` so the response echoes `X-Request-Id`.
- Order: set id → propagate → existing `TraceLayer` (and security/CORS as today). Exact Axum layer order documented in the implementation plan so the id exists when the span is created.

### Spans

Configure `TraceLayer::new_for_http().make_span_with(...)` so the root HTTP span records `request_id = %id` (from the request-id extension / header). Existing `tracing::*` calls inside handlers inherit that span, so JSON events include `request_id` (via current span / span list). Text format keeps readable spans.

Background tasks spawned **without** inheriting the request span (maintenance, mail sender, link checker) correctly omit HTTP `request_id`.

### Docs and product text

- `docs/INSTALL.md`: new **Logging** subsection (near Metrics / Advanced) for `LOG_FORMAT` and `LOG_FILE`; keep `RUST_LOG` where it is or cross-link.
- `.env.example`: commented examples.
- `docs/ARCHITECTURE.md`: one observability bullet — JSON/text, optional JSONL file, `X-Request-Id`.
- On ship: remove Now #9 from `docs/ROADMAP.md`, add a FEATURES bullet under the matching ops/observability section; bump `docs/MANAGED_SERVICE.md` “next” pointer past #9 if it still names it.

### Testing

- Unit: `MakeRequestId` accepts valid inbound ids, rejects/replaces invalid, mints when missing.
- Integration / light HTTP test: response has `X-Request-Id`; when request sends a valid id, response echoes it.
- Subscriber smoke (optional unit with a buffer writer): `LOG_FORMAT=json` produces a line parseable as JSON containing `request_id` when an instrumented span is entered — only if cheap without full server; otherwise rely on MakeRequestId + one axum test for the header.

### Ops notes

- Docker default stays text unless compose/env sets `LOG_FORMAT=json` (homelab/Cloud later).
- No rotation in-process; document that operators should rotate `LOG_FILE` externally if they use it.
- Do not log secrets (unchanged playbook rule).

## Success criteria

- With defaults, local/dev behaviour matches today (text stdout, no file).
- `LOG_FORMAT=json` → stdout is one JSON object per line.
- `LOG_FILE=/path` → that path grows with JSONL regardless of stdout format.
- Every HTTP response carries `X-Request-Id`; valid inbound ids are preserved; invalid ones are replaced.
- Request-scoped log lines carry `request_id`; INSTALL/ARCHITECTURE/FEATURES/ROADMAP stay in sync when the PR closes the item.
