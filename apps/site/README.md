# Linkdean marketing site

Static [Astro](https://astro.build/) site for `https://linkdean.app`. No server code: the build is plain HTML, CSS, fonts and images.

## Build

```bash
cd apps/site
pnpm install
pnpm build
```

The build writes straight into `deploy/public-beta/static/site/` (`outDir` in `astro.config.mjs`), which Caddy serves on the apex host. Commit that folder with any change to `apps/site`: CI rebuilds it and fails if the committed copy differs.

`pnpm dev` runs a local preview on `http://localhost:4321`.

Requires Node 22.23.4 or later (Astro 7) and pnpm 10, the same pnpm as `apps/web`.

## Content rules

- Features come from `docs/product/features.md`. No invented numbers, quotes or logos.
- Brand: `brand/GUIDELINES.md`. Logos, favicons and the mascot are copies of `brand/final/`, `brand/exports/web/` and `apps/web/public/mascot/`; fonts are copies of `apps/web/public/fonts/`.
- Links: sign-up and legal pages live on `https://app.linkdean.app` (`/register`, `/terms`, `/privacy`, `/acceptable-use`); self-hosting points at `https://docs.linkdean.app` and the install guide on GitHub.
- Contact and abuse reports: `contact@linkdean.app`, in the footer.

## Design system

The page is built with the taste-skill kit (v2.5.0). Its runtime lives in `public/assets/` (`system.css`, `system.js`, `ui/`, `motion/`, `layout/`, `icons/`): copies of the kit's files, never edited by hand. `src/styles/page.css` holds only the brand palette, the brand faces, the dark theme and the page CSS the kit's recipes ask for.

- Hero `centred`, nav `nav-mega`, close `close-word`, footer `footer-dark`, rhythm `surface-pairs`, buttons `arrow`.
- Brand exceptions, by request: Purple Pain is the accent (the kit avoids violet), and the type is the brand's Rajdhani and Satoshi in place of the dealt pairing.
- Product pictures are real: the hero shows a screenshot of the library, and the app frame and cards are live HTML of Linkdean's own screens (library list, command palette, library health, import, sharing, vault notifications, `docker compose`).
- Browser icons (Chrome, Firefox, Safari) come from the kit's `icons.mjs` (CC0 logos set); `public/assets/icons/icons-credits.json` lists them.
