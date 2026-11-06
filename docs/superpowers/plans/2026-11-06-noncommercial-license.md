# Non-commercial license Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Linkdean’s MIT project license with PolyForm Noncommercial 1.0.0 and document commercial contact `arsamsarabi@me.com`.

**Architecture:** Swap root `LICENSE` for the official PolyForm NC text plus a project header (`Required Notice` + commercial contact). Align README badge/section and workspace `Cargo.toml` `license` field. No runtime code changes.

**Tech Stack:** PolyForm Noncommercial 1.0.0 (SPDX `PolyForm-Noncommercial-1.0.0`); Cargo workspace metadata; README badges.

**Spec:** `docs/superpowers/specs/2026-11-06-noncommercial-license-design.md`

---

### File map

| File | Responsibility |
| --- | --- |
| `LICENSE` | Full PolyForm NC body + Linkdean header |
| `README.md` | Badge + License section |
| `Cargo.toml` | `workspace.package.license` SPDX id |
| Spec/plan under `docs/superpowers/` | Already written; include in same PR if uncommitted |

Do **not** change icon-credit MIT lines or `brand/node_render` ISC.

---

### Task 1: Replace `LICENSE`

**Files:**
- Modify: `LICENSE`

- [x] **Step 1:** Download the official license body (prefer PolyForm repo tag `1.0.0`):

```bash
curl -fsSL -o /tmp/PolyForm-Noncommercial-1.0.0.md \
  "https://raw.githubusercontent.com/polyformproject/polyform-licenses/1.0.0/PolyForm-Noncommercial-1.0.0.md"
```

- [x] **Step 2:** Write `LICENSE` as:

1. Project header (exact):

```text
Linkdean
Copyright (c) 2026 Arsam Sarabi

Required Notice: Copyright (c) 2026 Arsam Sarabi

Commercial use of this software requires a separate license.
Contact: arsamsarabi@me.com

---

```

2. Then the full downloaded PolyForm Noncommercial 1.0.0 text (keep the official title and URL line from that file).

- [x] **Step 3:** Verify:

```bash
rg -n "PolyForm Noncommercial|Required Notice|arsamsarabi@me.com|MIT License" LICENSE
```

Expected: PolyForm + Required Notice + email present; no `MIT License` grant text.

- [ ] **Step 4:** Commit (only if user asked): `docs: switch LICENSE to PolyForm Noncommercial 1.0.0`

---

### Task 2: README badge and License section

**Files:**
- Modify: `README.md` (badge ~L13, section ~L260–262)

- [x] **Step 1:** Replace badge:

```html
<a href="LICENSE"><img src="https://img.shields.io/badge/license-PolyForm%20NC-8458B3?style=flat-square" alt="PolyForm Noncommercial License" /></a>
```

- [x] **Step 2:** Replace `## License` body with:

```markdown
## License

[PolyForm Noncommercial License 1.0.0](LICENSE) © Arsam Sarabi.

Non-commercial use, modification, and redistribution (including forks) are allowed under that license. Commercial use — including selling Linkdean or derivatives — requires a separate license: **arsamsarabi@me.com**.

Prior MIT-tagged releases remain MIT for those trees; current `main` is PolyForm Noncommercial.
```

- [x] **Step 3:** Verify no project-level MIT claim:

```bash
rg -n "MIT|PolyForm|arsamsarabi@me.com" README.md
```

Expected: badge/section PolyForm; email present; no `license-MIT` or `[MIT](LICENSE)`.

- [ ] **Step 4:** Commit (only if user asked): `docs: update README for PolyForm Noncommercial`

---

### Task 3: Cargo workspace license metadata

**Files:**
- Modify: `Cargo.toml` (`[workspace.package]` `license`)

- [x] **Step 1:** Change:

```toml
license = "PolyForm-Noncommercial-1.0.0"
```

Leave `authors = ["Linkdean Contributors"]` unchanged (copyright notice lives in `LICENSE`).

- [x] **Step 2:** Sanity check metadata:

```bash
cargo metadata --no-deps --format-version 1 | rg -o '"license":"[^"]*"' | head
```

Expected: `"license":"PolyForm-Noncommercial-1.0.0"` (or equivalent) for workspace packages.

If Cargo/crates tooling rejects the SPDX id, set `license-file = "LICENSE"` instead and drop `license = ...` (document in PR).

- [ ] **Step 3:** Commit (only if user asked): `chore: set workspace license to PolyForm-Noncommercial-1.0.0`

---

### Task 4: Final verification

- [x] **Step 1:** Confirm untouched third-party MIT/ISC:

```bash
rg -n '"license": "ISC"|icons-credits' brand/node_render/package.json apps/site/public/assets/icons/icons-credits.json | head
```

Expected: still ISC / icon MIT credits (unchanged).

- [x] **Step 2:** Confirm project files:

```bash
rg -n "PolyForm-Noncommercial|PolyForm Noncommercial|arsamsarabi@me.com" LICENSE README.md Cargo.toml
```

- [x] **Step 3:** `graphify update .` after edits (repo has graphify-out).

- [x] **Step 4:** Stop — do not push/PR unless the user asks.
