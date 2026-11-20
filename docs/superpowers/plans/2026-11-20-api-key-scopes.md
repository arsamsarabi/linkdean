# API Key Scopes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Per-key read/write scopes by function group, enforced on REST and MCP.

**Architecture:** `scopes TEXT[]` on `api_keys`; catalogue + path map in `api_scopes.rs`; `AuthUser` enforces for `ldk_` tokens; MCP tools check resource scopes.

**Tech Stack:** Postgres, Axum `AuthUser`, existing API keys UI.

**Spec:** [docs/design/api-key-scopes.md](../../design/api-key-scopes.md)

---

### Task 1: Core + migration + DB

- [x] `migrations/061_api_key_scopes.sql`
- [x] `crates/core/src/api_scopes.rs`
- [x] Model + `create_api_key` / `user_and_scopes_from_api_key`
- [x] DB tests

### Task 2: Enforce + MCP + UI + docs

- [x] `AuthUser` scope enforcement
- [x] MCP `require_mcp_scope`
- [x] Settings checkboxes + schema
- [x] features / changelog / roadmap Phase A → B
- [x] HTTP scope tests
