# MCP archive text (Agent Skills Phase B)

**Status:** implementing.  
**Roadmap:** Agent Skills, next steps — Phase B (archive text).  
**Related:** [mcp-readonly.md](./mcp-readonly.md) (Phase A), [agent-skills.md](./agent-skills.md), `crates/core/src/page_text.rs` (`html_to_reader_markdown`).

## Intent

When an HTML archive copy exists, let an MCP-connected agent read **reader-mode Markdown** of that page without fetching the live URL.

## Goals

- New read-only MCP tool `get_archive_text` behind the same `EXPERIMENTAL_AGENT_SKILLS` gate.
- Reuse existing reader Markdown extraction (`html_to_reader_markdown`, capped at `PAGE_TEXT_MAX`).
- Same privacy stance as Phase A: no vault, respect share / Only-me visibility, no trash, no writes.
- Clear errors when there is no HTML copy, the copy is empty of readable text, or the bookmark is vault.

## Non-goals

- Embedding archive text into zip Agent Skills (size / packing; keep MCP-led).
- Serving PDF/file bodies or raw HTML over MCP.
- Vault unlock via API key / decrypting vault archives.
- Changing Meili page-text indexing (already ships for library search).

## Approaches considered

1. **MCP tool that reads the HTML file and returns Markdown** (chosen) — matches REST `as=md`, one place agents already connect.
2. Extend `get_bookmark` with an optional archive body — bloated default payloads; agents that only need metadata pay for large text.
3. Zip skill embedding — offline-friendly but packs explode; deferred.

## Design

### Tool: `get_archive_text`

| Input | Behaviour |
| --- | --- |
| `bookmark_id` (required) | Target bookmark |
| `format` (optional) | Only `html` is supported (default `html`). Other values → clear invalid-params error |

**Authz:** same as `get_archive_status` — load bookmark; refuse vault; refuse others' Only-me; otherwise require read access via existing patterns.

**Load:** `archive_row` for HTML; require `file_name` and done copy on disk. If `key_vault_id` is set, refuse (`vault not available via MCP`) even if the bookmark flag were wrong.

**Convert:** read file bytes as UTF-8 (lossy), `html_to_reader_markdown`. Empty → invalid-params with the same user-facing idea as REST (`No readable text in this copy.`).

**Response:**

```json
{
  "bookmark_id": "<uuid>",
  "format": "html",
  "markdown": "# …",
  "captured_at": "…"
}
```

No raw HTML, no download URLs, no PDF/file bytes.

### Docs / UI

- Guide tool table + “never included” line updated (reader text **is** available via this tool; raw archive bytes still never).
- MCP card: one bullet that archived page text is available when a copy exists.
- `features.md`, changelog `[Unreleased]`, roadmap Phase B removed (Phase C remains).

### Testing

- Happy path: finished HTML archive on disk → Markdown contains expected heading/paragraph.
- No archive / only PDF queued → not found or clear “no HTML archive”.
- Empty / chrome-only HTML → no readable text error.
- Vault bookmark → vault unavailable.
- Stranger / Only-me sharee → not found (no leak).
