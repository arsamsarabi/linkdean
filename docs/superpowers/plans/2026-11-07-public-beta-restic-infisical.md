# Public beta Infisical + restic — implementation plan

**Spec:** [2026-11-07-public-beta-restic-infisical-design.md](../specs/2026-11-07-public-beta-restic-infisical-design.md)

### Task 1: Spec + docs in repo

- [x] Design spec under `docs/superpowers/specs/`
- [x] `deploy/public-beta/BACKUP.md`
- [x] Update `MANAGED_SERVICE.md`, `ROADMAP.md` #13, pointer in `SELF_HOSTED.md`
- [x] Extend `deploy/public-beta/.env.example` with Infisical / restic key names

### Task 2: Render env from Infisical

- [x] `deploy/public-beta/scripts/render-env-from-infisical.sh`
- [x] Call it from release deploy before `pull-update.sh`

### Task 3: Nova restic job

- [x] Reference `deploy/public-beta/scripts/backup-linkdean.sh`
- [ ] Install on Nova under `~/novabot/restic/`, cron, `restic init` (host go-ahead)
- [ ] Restore drill documented / run once
