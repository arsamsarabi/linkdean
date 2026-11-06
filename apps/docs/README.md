# Linkdean docs site

Docusaurus site for [docs.linkdean.app](https://docs.linkdean.app).

## Content sync

Canonical operator docs live in the repo root `docs/` folder (`INSTALL.md`, `SELF_HOSTED.md`, `FEATURES.md`, `ARCHITECTURE.md`).

```bash
pnpm sync    # copies those files into apps/docs/docs/ with frontmatter + link rewrites
pnpm start   # sync + local preview
pnpm build   # sync + build + publish into deploy/public-beta/static/docs/
```

Commit `deploy/public-beta/static/docs/` with any content or theme change. CI and Deploy main rebuild and fail if that folder is stale (same pattern as `apps/site`).

**Do not edit** the generated markdown under `docs/self-hosting/`, `docs/features.md`, or `docs/architecture.md`. Change the sources in `/docs` and re-run `pnpm sync`.

Hand-written pages (e.g. `docs/intro.md`) are edited here.

## Stack

- Docusaurus 3.10 (classic, TypeScript) — see [installation docs](https://docusaurus.io/docs/installation)
- Node 20+
- pnpm (same major as `apps/web`)
