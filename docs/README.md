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
| [guides/agent-skills.md](./guides/agent-skills.md) | MCP connect and Agent Skill zips | Yes |
| [product/architecture.md](./product/architecture.md) | Stack and domain model | Yes |
| [product/roadmap.md](./product/roadmap.md) | Planned work | No (GitHub) |
| [product/ideas.md](./product/ideas.md) | Unprioritised scratch ideas | No |
| [design/organizations-and-teams.md](./design/organizations-and-teams.md) | Active product design | No |
| [design/agent-skills.md](./design/agent-skills.md) | Agent Skills: what it is and how it works | No |
| [design/archive.md](./design/archive.md) | Archive and downloads: first slice design | No |
| [design/archive-plan.md](./design/archive-plan.md) | Archive and downloads: implementation plan | No |
| [CHANGELOG.md](./CHANGELOG.md) | Release notes | No |

The docs site at [docs.linkdean.app](https://docs.linkdean.app) copies the published pages from here at each release, so edit them here. `api/openapi.json` is the API description it publishes, and `media/` holds the README's images.

Running linkdean.app (the public beta's hosting, deploys, backups and the docs and marketing sites' source) is kept in a separate private repository.
