# MCP Archive Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add MCP tool `get_archive_text` that returns reader-mode Markdown from an HTML archive copy.

**Architecture:** Handler in `mcp/tools.rs` mirrors `get_archive_status` authz, loads the HTML file via `archive_row` + `AppState::archive_dir`, converts with `html_to_reader_markdown`. Wire tool in `server.rs`; DTO in `dto.rs`. Docs/UI/roadmap in the same PR.

**Tech Stack:** Existing `rmcp` MCP stack, `linkdean_core::page_text`, Axum `AppState`.

**Spec:** [docs/design/mcp-archive-text.md](../../design/mcp-archive-text.md)

---

### Task 1: Tool + tests

**Files:**
- Modify: `crates/api/src/mcp/dto.rs`, `tools.rs`, `server.rs`, `mod.rs` (tests)
- Set `ARCHIVE_DIR` in MCP test `state()` to the temp dir

- [ ] Add `McpArchiveText` DTO
- [ ] Implement `get_archive_text` in tools
- [ ] Register tool on `LinkdeanMcp`
- [ ] Tests: happy path, no HTML, empty text, vault, unauthorized
- [ ] `cargo fmt` in rust:1.94 container; run MCP archive-text tests with `DATABASE_URL`

### Task 2: Docs + UI + roadmap

**Files:**
- Modify: `docs/guides/agent-skills.md`, `docs/product/features.md`, `docs/CHANGELOG.md`, `docs/product/roadmap.md`, `docs/design/mcp-readonly.md` (Phase B note), `docs/design/agent-skills.md`, `apps/web/src/components/import-export/McpConnectCard.tsx` (+ test if asserted)

- [ ] Guide tool table + never-included wording
- [ ] Features / changelog / roadmap Phase B removal
- [ ] MCP card bullet
- [ ] Mark design status shipped when green

### Task 3: PR

- [ ] Push branch, open one PR (design + plan + code)
