# Linkdean docs site

Docusaurus site for [docs.linkdean.app](https://docs.linkdean.app).

## Content sync

Canonical operator docs live in the repo root `docs/` folder (`INSTALL.md`, `SELF_HOSTED.md`, `FEATURES.md`, `ARCHITECTURE.md`).

```bash
pnpm sync    # copies those files into apps/docs/docs/ with frontmatter + link rewrites
pnpm start   # sync + local preview
pnpm build   # sync + build + publish into deploy/public-beta/static/docs/
```

`pnpm build` writes `deploy/public-beta/static/docs/`. Webpack asset hashes differ between macOS and Linux, so CI builds on Ubuntu and uploads a `docs-site-static` artifact instead of requiring a byte-identical committed tree (unlike `apps/site`, where Astro output matches). On the guest, either unpack that artifact or run `pnpm --dir apps/docs build` after pull.

**Do not edit** the generated markdown under `docs/self-hosting/`, `docs/features.md`, or `docs/architecture.md`. Change the sources in `/docs` and re-run `pnpm sync`.

Hand-written pages (e.g. `docs/intro.md`) are edited here.

## Stack

- Docusaurus 3.10 (classic, TypeScript) — see [installation docs](https://docusaurus.io/docs/installation)
- Node 20+
- pnpm (same major as `apps/web`)
