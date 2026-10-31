# Linkdean identity — handover

## Direction
**Snap Chain** — two open links mid-interlock (mouths ajar = snap). Approved from concept D (`d-snap-chain-v4`).

## Palette (from `apps/web/src/theme.ts`)
- Purple Pain `#8458B3` — primary
- Ice Cold `#A0D2EB` — accent mono
- Freeze `#E5EAF5` · Heavy `#A28089` · Night `#1A1618`

## Deliverables
| Path | Contents |
|---|---|
| `brand/final/` | Masters: symbol, small cut, wordmark, horizontal, stacked (black/color/white) |
| `brand/exports/` | One-colour, square, favicon, app-icon SVG variants |
| `brand/exports/web/` | favicon.ico/svg, apple-touch, PWA icons, manifest, head snippet |
| `brand/exports/png/` | Raster previews |
| `brand/board/linkdean-presentation.html` | Final presentation (5 slides) |
| `brand/GUIDELINES.md` | Compact usage guide |
| `apps/web/public/` | Wired favicons + `linkdean-symbol.svg` / `linkdean-logo.svg` |

## App wiring
- `apps/web/index.html` — favicon / manifest / theme-color
- `AppShell` + `LoginPage` — symbol beside title

## Notes
- Wordmark = Rajdhani Bold outlined via opentype.js (not live `<text>`)
- Small-size cut = thicker rings for ≤32 px
- No trademark clearance performed
- Prefer `linkdean-symbol-small` / favicon assets below 24 px
