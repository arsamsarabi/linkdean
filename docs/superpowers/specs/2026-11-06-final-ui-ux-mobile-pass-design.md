# Final UI/UX and mobile pass

Design for roadmap item **#8** before the free public beta. Agreed approach: three reviewable PRs (mascot moments → mobile pass → light polish).

## Goals

1. Moment screens carry the Linkdean mascot (Dean) with clear placement rules.
2. Every primary page is usable on a phone, not only the nav.
3. Light hierarchy/spacing polish on high-traffic surfaces after (1) and (2).

## Non-goals

- Full visual redesign of the app chrome, cards, or settings.
- Mascot in header, sidebar, cards, dashboards, or settings.
- Welcome-email avatar badge (no welcome template today; security emails stay plain). Parked as Remaining on #8.
- Inventing new success modals. Do not decorate security-critical overlays (e.g. recovery key). Asset `deen-approves.webp` ships for a future celebratory modal only.

## Mascot rules (from ROADMAP)

- Moments only: empty library, empty folder, no search results, empty Deleted, nothing shared yet (page-level), 404, first-run onboarding, sign-in side panel on wide screens.
- One character per screen, behind the main action.
- WebP at 2× display size, lazy-loaded, `alt=""`, soft light backdrop so the dark robe reads on dark theme.

### Art map

| Moment | File under `/mascot/` |
| --- | --- |
| Empty library / vault empty | `dean-standing.webp` |
| Empty folder | `dean-thinking.webp` |
| No search / filter results | `dean-looking-up.webp` |
| Empty Deleted | `dean-smiling.webp` |
| Shared page fully empty | `dean-looking-up.webp` |
| 404 | `dean-thinking.webp` |
| Onboarding | `dean-standing.webp` |
| Login wide side panel | `dean-standing.webp` |
| Future success modal | `deen-approves.webp` |

Source PNGs live in untracked `brand/graphics/`. Deliverables are committed under `apps/web/public/mascot/`.

### EmptyState

Optional `illustration` replaces the ThemeIcon. Soft radial backdrop. Icon-only EmptyState remains for non-moment empties (e.g. Shared section rows when the page is only partly empty).

### Shared page

When invites, shared-with-me, and shared-by-me are all empty after load: one page-level EmptyState with mascot. Otherwise keep compact section empties without mascot.

### Login

From `md` up: form + mascot column. Below `md`: form only.

## Mobile pass (PR2)

Fix breakage and cramped layouts only: drawers full-width on `base`, tables scroll or stack, library chrome wraps, auth pages single-column comfort. No new visual language.

## Polish (PR3)

Time-boxed spacing/copy consistency on library, dashboard empty, shared. Then move #8 to FEATURES with Remaining for welcome email and any parked Known issues.

## PR split

1. **feat/ui-mascot-moments** — assets, EmptyState, wire moments, 404, login/onboarding.
2. **feat/ui-mobile-pass** — responsive fixes.
3. **feat/ui-ux-polish** — polish + roadmap close-out.
