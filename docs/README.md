# Linkdean docs

Canonical markdown for operators, contributors, and the public docs site.

## Layout

| Path | Audience | Published on docs.linkdean.app? |
| --- | --- | --- |
| [self-hosting/quick-start.md](./self-hosting/quick-start.md) | Operators installing a released image | Yes |
| [self-hosting/environment.md](./self-hosting/environment.md) | Env-var reference (single source of truth) | Yes |
| [self-hosting/operations.md](./self-hosting/operations.md) | Secrets, proxy, email, SSO, backup, upgrades, local Compose | Yes |
| [product/features.md](./product/features.md) | What ships today | Yes |
| [guides/fast-capture.md](./guides/fast-capture.md) | Extension, bookmarklet, share, Shortcuts | Yes |
| [guides/agent-skills.md](./guides/agent-skills.md) | Folders and tags as Agent Skills | Yes |
| [product/architecture.md](./product/architecture.md) | Stack and domain model | Yes |
| [product/roadmap.md](./product/roadmap.md) | Planned work | No (GitHub) |
| [product/ideas.md](./product/ideas.md) | Unprioritised scratch ideas | No |
| [ops/managed-service.md](./ops/managed-service.md) | Homelab public beta + hosted notes | No |
| [design/organizations-and-teams.md](./design/organizations-and-teams.md) | Active product design | No |
| [design/agent-skills.md](./design/agent-skills.md) | Agent Skills: what it is and how it works | No |
| [CHANGELOG.md](./CHANGELOG.md) | Release notes | No |

The Docusaurus app in `apps/docs` copies the published pages via `pnpm sync`. Edit sources here, not under `apps/docs/docs/`.

Live ops for the public beta guest live under [`deploy/public-beta/`](../deploy/public-beta/) (see especially [BACKUP.md](../deploy/public-beta/BACKUP.md)).
