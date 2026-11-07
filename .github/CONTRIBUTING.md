# Contributing to Linkdean

Thanks for wanting to help. Linkdean is a small, self-hosted bookmarks server: a Rust API, a React web app, Postgres and Meilisearch, shipped as one Docker image. Small PRs, clear UX and boringly solid tooling beat clever abstractions.

You don't need to be an expert. Bug reports, docs fixes and good questions all count.

Linkdean is maintained by one person. Issues and pull requests are welcome, and I'll respond as time allows.

## Before you start

- Skim the [README](../README.md) so you know what Linkdean is (and isn't), and [docs/FEATURES.md](../docs/FEATURES.md) for what already ships.
- Check [docs/ROADMAP.md](../docs/ROADMAP.md) and [existing issues](https://github.com/arsamsarabi/linkdean/issues) before starting something new.
- For larger ideas, open an issue first so we can agree on the approach.

## Project layout

| Path | What lives there |
| --- | --- |
| `crates/api` | Axum HTTP API; also serves the built web app |
| `crates/core` | Domain types, validation, crypto, import/export |
| `crates/db` | Postgres queries (sqlx); integration tests in `crates/db/tests` |
| `crates/search` | Meilisearch indexing and queries |
| `crates/worker` | Background jobs: previews, screenshots, broken-link checks |
| `crates/mail` | SMTP configuration and sending |
| `apps/web` | React + Vite web app (TanStack Router, Mantine) |
| `migrations` | SQL migrations, run by the API at startup |
| `deploy/docker` | Production image, production and dev Compose files |
| `docs` | Architecture, features, roadmap, install and self-hosting guides |

More detail: [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md), [crates/AGENTS.md](../crates/AGENTS.md) and [apps/web/AGENTS.md](../apps/web/AGENTS.md).

## Development setup

**Requirements**

- Docker with the Compose plugin
- For running checks on the host: Rust `1.94` (what CI and the image use), Node `22`, and `pnpm` (enable it with `corepack enable`)
- Optional: [`cargo-nextest`](https://nexte.st) for faster Rust test runs

```bash
git clone https://github.com/arsamsarabi/linkdean.git
cd linkdean
docker compose -f deploy/docker/docker-compose.dev.yml up --build
```

Open http://localhost:5173. The dev stack needs no `.env`: it uses well-known development secrets and binds its ports to `127.0.0.1`. Web changes hot-reload, and the API rebuilds when `crates/` or `migrations/` change. The first run compiles the API, so give it a few minutes.

To run the API and web app on the host instead, see [Development](../README.md#development) in the README.

## Checks

CI runs all of these on every push and pull request. Run the ones your change touches before you open a PR.

| Command | What it checks |
| --- | --- |
| `cargo fmt --all -- --check` | Rust formatting |
| `cargo nextest run --workspace` | Rust tests (`cargo test --workspace` also works, more slowly) |
| `cargo test --workspace --doc` | Rust doctests (nextest skips them) |
| `cd apps/web && pnpm format:check` | Prettier |
| `cd apps/web && pnpm lint` | oxlint |
| `cd apps/web && pnpm test` | Vitest |
| `cd apps/web && pnpm build` | Route generation, TypeScript and production build |
| `python3 deploy/scripts/sync_install_seccomp.py --check` | The Chrome seccomp profile copied into `docs/INSTALL.md` matches `deploy/docker/chrome-seccomp.json` |

### Database tests

Most Rust tests live in `crates/db/tests` and need a real Postgres. **Without `DATABASE_URL` they skip and still pass**, printing `skipping ... DATABASE_URL unset`. Point them at a throwaway database, never your dev data:

```bash
docker run -d --name linkdean-test-pg -p 55432:5432 \
  -e POSTGRES_USER=linkdean -e POSTGRES_PASSWORD=linkdean -e POSTGRES_DB=linkdean_test \
  postgres:16-alpine

DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo nextest run --workspace
```

Tests apply migrations themselves. Check that no `skipping` line appears, not only that the run passed.

nextest runs every test in its own process, in parallel. A test that changes instance-wide state (platform settings, or claiming every pending row in a shared table) must join the `db-global-state` group in [`.config/nextest.toml`](../.config/nextest.toml), or it will race other tests.

## How we like changes

- **Small and focused.** One problem per PR when you can.
- **Match the neighborhood.** Follow the style of nearby code. Rust is formatted with `rustfmt`; the web app with Prettier and linted with oxlint.
- **Keep it lean.** Avoid new dependencies unless they clearly earn their weight.
- **Leave a check behind.** Non-trivial logic gets a test that fails if the behavior regresses: a Rust unit or `crates/db/tests` integration test, or a Vitest + React Testing Library test in the web app. Trivial one-liners don't need ceremony.
- **Migrations are append-only.** Add a new numbered file in `migrations/` using the next free number. Never edit a migration that has already shipped.
- **Docs move with the code.**
  - A user-visible change adds or updates its bullet in [docs/FEATURES.md](../docs/FEATURES.md). If it finishes a roadmap item, move that item out of [docs/ROADMAP.md](../docs/ROADMAP.md).
  - A new or changed environment variable, port, volume or deployment step updates [docs/INSTALL.md](../docs/INSTALL.md) and [docs/SELF_HOSTED.md](../docs/SELF_HOSTED.md). Every variable the docs mention must also reach the API in `deploy/docker/docker-compose.yml`.
- **Known defects live in the code.** If you find a problem you won't fix in your PR, leave a `Known issue:` comment above the code it concerns, describing the problem and a suggested fix.
- **American spelling** in code, UI text, API fields and docs ("organization", "color").

Note user-facing changes in [CHANGELOG.md](../docs/CHANGELOG.md).

## Pull requests

1. Branch from `main`.
2. Make your change; keep commits readable, with messages that say why.
3. Run the [checks](#checks) your change touches.
4. Open a PR against `main` with a short description of **why**, not just what. Include screenshots for UI changes.
5. Link related issues, if any.

CI must be green. I'll review as soon as I can, and feedback is about the code, not you.

## Releases

For maintainers:

1. Set the new version in `Cargo.toml` (`[workspace.package]`) and `apps/web/package.json`, and refresh `Cargo.lock` with `cargo update -w`.
2. Update `docs/CHANGELOG.md`, commit and push to `main`.
3. Tag and push: `git tag -a vX.Y.Z -m "Linkdean X.Y.Z" && git push origin vX.Y.Z`.

The release workflow reruns CI, refuses a tag that doesn't match both versions, pushes `ghcr.io/arsamsarabi/linkdean:X.Y.Z` (plus `X.Y` and `latest`) and creates the GitHub release.

## Reporting bugs

A good bug report includes:

- The Linkdean version (the image tag or `LINKDEAN_VERSION`), or the commit if you build from source
- How you run it (published image, Compose from a checkout, or on the host) and which optional features are on (SMTP, SSO, Chrome screenshots)
- Your browser, for web app issues
- What you did, what you expected and what happened
- Relevant logs (`docker compose logs api`) and the output of `/api/ready`, with secrets removed

## Security issues

Please don't open a public issue for a vulnerability. See [SECURITY.md](./SECURITY.md) for how to report one privately and what's in scope.

## Code of conduct

This project follows the [Contributor Covenant](./CODE_OF_CONDUCT.md). Be respectful, assume good intent, and give people the benefit of the doubt. To report an incident, use the contact listed in that document.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](../LICENSE), the same as the rest of the project.

---

Questions are welcome on the PR or issue. Thanks again for contributing.
