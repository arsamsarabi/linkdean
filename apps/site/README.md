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

- Features come from `docs/FEATURES.md`. No invented numbers, quotes or logos.
- Brand: `brand/GUIDELINES.md`. Logos, favicons and the mascot are copies of `brand/final/`, `brand/exports/web/` and `apps/web/public/mascot/`; fonts are copies of `apps/web/public/fonts/`.
- Links: sign-up and legal pages live on `https://app.linkdean.app` (`/register`, `/terms`, `/privacy`, `/acceptable-use`); self-hosting points at `https://docs.linkdean.app` and the install guide on GitHub.
- Contact and abuse reports: `contact@linkdean.app`, in the footer.
