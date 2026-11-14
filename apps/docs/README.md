# Linkdean docs site

Docusaurus site for [docs.linkdean.app](https://docs.linkdean.app).

## Content sync

Canonical operator docs live under the repo `docs/` tree (`self-hosting/`, `product/`, `guides/`). See [docs/README.md](../../docs/README.md).

```bash
pnpm sync    # copies those files into apps/docs/docs/ with frontmatter + link rewrites
pnpm start   # sync + local preview
pnpm build   # sync + build into build/
```

`pnpm build` writes `build/`, which is not committed. The Release workflow builds the docs and the marketing site into one Caddy image (`deploy/docker/Dockerfile.static`, pushed as `ghcr.io/arsamsarabi/linkdean-static`) that serves `docs.linkdean.app`.

**Do not edit** the generated markdown under `docs/self-hosting/`, `docs/guides/`, `docs/features.md`, or `docs/architecture.md`. Change the sources in `/docs` and re-run `pnpm sync`.

Hand-written pages (e.g. `docs/intro.md`) are edited here.

## Stack

- Docusaurus 3.10 (classic, TypeScript) — see [installation docs](https://docusaurus.io/docs/installation)
- Node 20+
- pnpm (same major as `apps/web`)
