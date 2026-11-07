# Public beta on Nova — remaining go-live plan

> Slices 1–4 (deploy skeleton, guest + Tunnel + app, marketing site, docs site) are shipped. This plan covers **slice 5 only**: go-live hardening before announce.

**Goal:** Close roadmap #13 go-live checklist so open sign-up can be announced. Spec: [public-beta-homelab-design](../specs/2026-11-06-public-beta-homelab-design.md). Operator checklist: [MANAGED_SERVICE.md](../../MANAGED_SERVICE.md#public-beta-on-a-home-server).

**Already live:** `app.linkdean.app`, [docs.linkdean.app](https://docs.linkdean.app), [linkdean.app](https://linkdean.app) (including `contact@linkdean.app`).

---

### Task 1: Transactional email + DNS auth

- [ ] Pick a transactional SMTP provider; set `SMTP_*` / `MAIL_FROM` on the guest.
- [ ] SPF, DKIM, and DMARC on `linkdean.app`.
- [ ] Smoke: password reset / verify-email land outside spam for a real inbox.

### Task 2: Sign-up misuse controls on the guest

- [ ] Set `TURNSTILE_SITE_KEY` / `TURNSTILE_SECRET_KEY`.
- [ ] Set `REQUIRE_VERIFIED_EMAIL=true` and `BLOCK_DISPOSABLE_EMAILS=true`.
- [ ] Confirm registration stays open (`registration_open`).

### Task 3: Chrome / LAN isolation

- [ ] Guest firewall (or equivalent) so the Chrome network cannot reach the home LAN.
- [ ] Keep Chrome optional until needed; private-address blocking remains on in the app.

### Task 4: Backups and monitoring

- [ ] Nightly off-site copy of Postgres dump + avatars/previews volumes.
- [ ] One restore tested end to end.
- [ ] External uptime check with phone alert.

### Task 5: Announce gate

- [ ] Confirm ISP terms allow serving a public site.
- [ ] Confirm `LOG_FORMAT=json` on the public API (Compose default in `deploy/public-beta`).
- [ ] Mark roadmap #13 **Done** and announce only when every item above is green.
