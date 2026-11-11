## Why

<!-- The problem this solves or the reason for the change. Link the issue if there is one: "Closes #123". -->

## What changed

<!-- A short summary. Call out anything a reviewer should look at closely. -->

## How it was tested

<!-- Commands you ran, manual steps, and screenshots or a short recording for UI changes. -->

## Checklist

- [ ] Rust: `cargo fmt --all -- --check` and `cargo nextest run --workspace` with `DATABASE_URL` set (no `skipping ... DATABASE_URL unset` lines)
- [ ] Web: `pnpm format:check`, `pnpm lint`, `pnpm test` and `pnpm build` in `apps/web`
- [ ] Non-trivial logic has a test that fails without the change
- [ ] New migrations use the next free number; no shipped migration was edited
- [ ] A test that changes instance-wide state is in the `db-global-state` group in `.config/nextest.toml`
- [ ] User-visible change: `docs/product/features.md` updated (and `docs/product/roadmap.md` if it finishes an item)
- [ ] New or changed env var, port, volume or deploy step: `docs/self-hosting/quick-start.md` / `environment.md` / `operations.md` and `deploy/docker/docker-compose.yml` updated as needed
- [ ] User-facing change noted in `docs/CHANGELOG.md`
- [ ] Defects left unfixed are marked with a `Known issue:` comment

<!-- Untick or delete items that don't apply. Security fixes: don't describe the vulnerability here; see .github/SECURITY.md. -->
