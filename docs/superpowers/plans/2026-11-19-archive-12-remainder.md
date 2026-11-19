# Archive #12 remainder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline) or subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Close the remaining roadmap item 12 slices that can ship without a billing system: hide archive UI when disabled, throttled auto-archive on import, full export zip with archives; explore embedded media; park version history and per-tier limits with clear roadmap wording.

**Architecture:** Reuse `archive_new_bookmark`, `archives_enabled`, and `zip_stored`. Expose `archives_enabled` on public auth config for the web. Export gains `include_archives=1` → zip of JSON + archive files. Import finish queues archives with paced wakes.

**Tech Stack:** Rust API/db/core/worker, React web, Postgres, existing archive volume.

**Spec:** `docs/design/archive.md` (Later + Markdown already shipped). Decisions locked in this plan where the design is silent.

## Global Constraints

- Branch: `feat/archive-12-remainder` in `.worktrees/archive-12-remainder`. Never commit `main`.
- Commit each green chunk; push; `GH_TOKEN=$(gh auth token --user arsamsarabi)` for gh.
- `cargo test -j 2`; DB tests need `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test`.
- Rustfmt in `rust:1.94` container only. Web: prettier + `tsc -b` on changed files.
- Do not invent billing/tiers. Do not commit `graphify-out/`, `ideas.md`, `brand/graphics/`.
- `graphify update .` after code edits (worktree root).

## Review Focus

1. Archive menu/settings hidden when `archives_enabled=false`, but existing Open/Download still work.
2. Import with `archive_new` queues without flooding Chrome (paced wake).
3. Export zip includes done archives; locked vault copies skipped like JSON plaintext.
4. Quota still enforced on import-queued archives.
5. Per-tier and version history remain explicitly unshipped on the roadmap.

## File map

| File | Responsibility |
| --- | --- |
| `crates/api` auth config | Expose `archives_enabled` |
| `apps/web` archive menus/settings | Gate write UI on flag |
| `crates/api` import_export | After import, queue archives; export zip |
| `crates/core` zip / export | Zip builder with archive entries |
| `crates/worker` mhtml (optional) | Plain media explore / fetch |
| `docs/product/roadmap.md` etc. | Ship notes; park L items |

## Rulings (product)

- **Hide UI:** Hide Archive…, bulk Archive, Archive again, Delete archive, Settings Archive controls when off. Keep Open archive / Download / Markdown for existing copies.
- **Import:** Only newly created bookmark ids from the import apply; throttle by waking the archive worker once after a batch queue (same as bulk), not per-row sleep storms. Cap queued per import to avoid unbounded queue — use existing quota/failed-row behaviour.
- **Export:** When `include_archives=1`, response is `application/zip` with `export.json` (or netscape `.html`) plus `archives/{bookmark_id}/{format}.{ext}` for `done` rows the user can read. Skip vault archives unless vault unlocked (decrypt in memory like serve).
- **Media:** Spike in-tree; if a small safe path exists (absolute http(s) `src` on video/audio/source, not m3u8/blob), fetch under remaining item budget and inline; else document findings and leave on roadmap.
- **Version history:** Out of this PR (L schema). Stay on roadmap.
- **Tiers:** Blocked on billing. Roadmap sub-bullet notes dependency.

---

### Task 1: Expose `archives_enabled` + hide write UI

- [ ] Test: auth config includes `archives_enabled`; web hides Archive… when false.
- [ ] Implement API + web gates.
- [ ] Commit + push.

### Task 2: Archive on import when `archive_new`

- [ ] Test: import with setting on queues archive rows; off does not; Settings copy updated.
- [ ] Implement queue after successful creates + single wake.
- [ ] Commit + push.

### Task 3: Export include archives (zip)

- [ ] Test: flag off = same as today; on = zip with json + archive bytes; access rules.
- [ ] Implement API + UI checkbox.
- [ ] Commit + push.

### Task 4: Embedded media explore

- [ ] Spike + either minimal implement with tests or write findings into design Later + keep roadmap bullet.
- [ ] Commit + push.

### Task 5: Docs + park L items + PR

- [ ] Update roadmap/features/changelog/public roadmap; open PR; drive CI green.
