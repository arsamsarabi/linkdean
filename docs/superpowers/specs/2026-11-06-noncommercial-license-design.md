# Non-commercial license (PolyForm)

Agreed 2026-11-06. Replace MIT with **PolyForm Noncommercial License 1.0.0** for Linkdean.

## Problem

Linkdean is currently MIT, which allows unrestricted commercial use, sale, and sublicensing. The product goal for free public beta is **source-available, non-commercial**: people may self-host and fork for personal / education / research / hobby use, but may not use it commercially or resell Linkdean or derivatives without a separate commercial grant.

## Goals

- Public grant limited to **non-commercial** purposes (PolyForm NC definitions).
- **Forks and redistribution allowed** under the same terms (with notices).
- **No commercial use** and no selling of the software or derivatives without a separate license from the copyright holder.
- Short **commercial-use contact** path in README (and a Required Notice / header on `LICENSE`).
- Update project metadata so crates and the README badge match the new license.
- Keep copyright holder **Arsam Sarabi** (current `LICENSE` notice).

## Non-goals

- Relicensing past published commits or Git tags that were MIT (those snapshots remain MIT for that tree).
- Drafting a custom paid commercial agreement text (contact only for now).
- Changing third-party asset licenses (e.g. icon credits that say MIT).
- Contributor License Agreement / paperwork beyond updating the project license files.
- Legal advice; this is an engineering/docs change using a published standard license.

## Approaches considered

| Approach | Pros | Cons |
| --- | --- | --- |
| **A. PolyForm Noncommercial 1.0.0** (chosen) | Standard NC text; allows non-commercial forks; clear NC definitions | Not OSI “open source”; GitHub may show “Other” |
| B. Custom proprietary NC | Exact “no resell forks” wording | Drafting risk; unrecognized |
| C. MIT + Commons Clause | Familiar base | Weaker / controversial fit for strict NC |

## Design

### License text

- Replace root [`LICENSE`](../../../LICENSE) with the full **PolyForm Noncommercial License 1.0.0** text from [polyformproject.org](https://polyformproject.org/licenses/noncommercial/1.0.0).
- Prepend a short project header, e.g.:

  ```text
  Linkdean
  Copyright (c) 2026 Arsam Sarabi

  Required Notice: Copyright (c) 2026 Arsam Sarabi

  Commercial use of this software requires a separate license.
  Contact: arsamsarabi@me.com
  ```

  then the PolyForm body (or URL + full text as customary).

### README

- Badge: non-commercial / PolyForm (not MIT).
- `## License` section: name the license, link `LICENSE`, note non-commercial + forks under same terms, commercial contact line.
- Fix copyright line from “Linkdean Contributors” to **Arsam Sarabi** to match `LICENSE` (unless dual notice is intentionally added later).

### Cargo workspace

- Root [`Cargo.toml`](../../../Cargo.toml): set `license` to the SPDX id **`PolyForm-Noncommercial-1.0.0`** (workspace crates already use `license.workspace = true`).
- If crates.io / tooling rejects that id in practice, fall back to `license-file = "LICENSE"` and document in the PR.

### Out of scope files

- Do not change `brand/node_render` ISC or third-party icon MIT credits.
- Do not rewrite historical docs that quote env var names containing “license” unrelated to copyright.

### Commercial path

- Public grant remains NC only.
- README + LICENSE header point to **arsamsarabi@me.com**.

### Relicensing note (docs only, one sentence)

- README license blurb may note that prior MIT-tagged releases remain under MIT for those trees; current `main` is PolyForm NC.

## Success criteria

- Root `LICENSE` is PolyForm NC 1.0.0 with Required Notice + commercial contact.
- README badge and License section match; no project-level “MIT” claim for Linkdean itself.
- Workspace `license` metadata matches.
- Commercial contact is **arsamsarabi@me.com**.

## Open items

None — commercial email provided 2026-11-06.
