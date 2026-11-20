# Legal review — counsel brief

**Status:** deferred — removed from the ranked product roadmap (2026-11-20); brief kept for when counsel is engaged. Live Terms/Privacy are not changed by this doc.  
**Audience:** external counsel (England & Wales).  
**Operator:** Arsam Sarabi, an individual (not a company). Contact: `contact@linkdean.app` / `privacy@linkdean.app` / `abuse@linkdean.app`.

Prepared so counsel can review the managed service documents **before anything costs money** (paid plans). Eng will apply agreed edits, bump versions, and re-prompt acceptance. Prefer plans/pricing next; re-engage this brief before charging.

---

## 1. Ask

Please review:

1. [Terms of Service](https://linkdean.app/terms) (includes Acceptable Use at `#acceptable-use`)
2. [Privacy Policy](https://linkdean.app/privacy)

**Focus**

- Fitness for a UK consumer / small-business bookmarking SaaS in **public beta**, still free, with **paid plans planned**.
- Operator as an **individual**, not a company; liability cap (£100 or amounts paid in prior 12 months); governing law **England and Wales**.
- Accuracy and completeness of privacy disclosures for the **current** managed stack (not future Cloud Run / multi-region unless noted).
- Gaps before charging money (e.g. consumer rights, cancellation, pricing presentation — even if pricing copy is not in these pages yet).
- Whether children’s / age language is adequate.
- Whether content licence, archives, vault, sharing, abuse reporting, and API / extension / agent tooling disclosures are adequate.

**Out of scope for this review (unless you flag them as blockers)**

- Drafting paid-plan or checkout Terms (no paid product yet).
- Organizations / teams / SSO / SCIM legal (roadmap later).
- Self-hosters’ own Terms (open-source app; operators set their own `TERMS_*` URLs).
- Marketing site or docs site copy beyond these two pages.
- ICO fee registration mechanics (operator action; see §5).

---

## 2. Current published version

| Item | Value |
| --- | --- |
| Version date shown on both pages | **2026-11-17** |
| App re-accept trigger | `TERMS_VERSION=2026-11-17` on the managed guest (`deploy/public-beta/docker-compose.yml` in the infra repo) |
| Source files (infra repo) | `apps/site/src/pages/terms.astro`, `privacy.astro`, framed by `apps/site/src/layouts/Legal.astro` (`const VERSION`) |
| App behaviour | App ships **no** legal body text. It reads `TERMS_VERSION`, `TERMS_URL`, `PRIVACY_URL`. Material Terms changes: bump version in Legal layout **and** compose together so every account gets one “Our terms changed” prompt. |

Live URLs: https://linkdean.app/terms · https://linkdean.app/privacy  
App redirects: `/terms`, `/privacy`, `/acceptable-use` → those URLs (`/acceptable-use` → Terms `#acceptable-use`).

---

## 3. Document maps

### Terms of Service

| § | Title | Notes for counsel |
| --- | --- | --- |
| 1 | Who we are and these Terms | Individual operator named |
| 2 | The service | Bookmarking / organising / sharing |
| 3 | Public beta and availability | No uptime/retention promise; export recommended |
| 4 | Your account | Credentials, API keys, suspension |
| 5 | Your content | User owns content; limited licence to operate; **archives**: fetched on user’s behalf, not on public lists, sharees only |
| 6 | Acceptable use | Illegal/harmful; protecting the service (incl. SSRF-style private network abuse); enforcement |
| 7 | Reporting abuse | In-app Report + `abuse@linkdean.app`; archives included |
| 8 | Ending your account | 7-day grace; aligns with Privacy |
| 9 | Intellectual property | Software/branding vs user content |
| 10 | Disclaimers and limitation of liability | As-is; £100 / 12 months paid cap |
| 11 | Contact | contact@ / abuse@ |
| 12 | Changes | Version date; continued use; material change notice efforts |
| 13 | Governing law | England and Wales |
| 14 | General | Severability, waiver, no partnership |

### Privacy Policy

| § | Title | Notes for counsel |
| --- | --- | --- |
| 1 | Who is responsible | Same individual controller; privacy@ |
| 2 | What we store | Account, content, activity, **vault** (passphrase never stored) |
| 3 | Cookies and similar storage | Session + OIDC cookies; no ad/analytics cookies; local/session UI prefs |
| 4 | Page view statistics | Self-hosted GoatCounter; DNT/GPC; app path scrubbing |
| 5 | Why + legal bases | Contract / LIs / legal obligation / consent |
| 6 | Security and abuse | IP + request info; in-memory rate limits by IP |
| 7 | Who else processes | iCloud Mail, Cloudflare (+ Turnstile), SSO IdPs, Meilisearch (same instance) |
| 8 | International transfers | Generic; ask for host location detail if needed |
| 9 | Public and shared content | + abuse report contents |
| 10 | Browser extension | What is sent on save; saved-page icon hashes; API key lifecycle |
| 11 | Retention | See §4 fact table |
| 12 | Choices and rights | Export, delete, UK ICO complaint |
| 13 | Contact | privacy@ / abuse@ |
| 14 | Children | Not intended where parental consent required |
| 15 | Changes | Version date |

---

## 4. Factual claims eng treats as source-of-truth

Re-verify against code before publishing counsel’s edits. Numbers below match the Privacy Policy as of version **2026-11-17**.

| Claim in Privacy / Terms | Eng source |
| --- | --- |
| Session cookie `linkdean_session` lasts **30 days** | `SESSION_DAYS` default `30` in API config |
| Account permanently removed **7 days** after delete request (cancellable) | `ACCOUNT_DELETION_GRACE_DAYS = 7` |
| Deleted items purged after **30 days**, or user setting **1–3650 days** | Trash retention / settings range |
| Individual bookmark-open rows kept **30 days**; open count + last open until bookmark/account gone | `purge_old_bookmark_opens` (maintenance) |
| Sent-email copies **7 days**, **30** if delivery failed | Email outbox retention |
| Passkeys: public key, name, created/last used; private key on device | Passkey / WebAuthn flow |
| UI prefs in local/session storage only | Web app storage keys |
| Page stats via self-hosted GoatCounter; app does not send referrer | `GOATCOUNTER_URL`; referrer policy |
| Cloudflare in front of marketing, app, docs | Live DNS / proxy; `__cf_bm` may appear |
| Turnstile on sign-up | Sign-up handler + widget |
| Email via Apple iCloud Mail | Managed SMTP config |
| Extension may send note, tags, folder with a save | Extension request body |
| Archives never on public lists; only owner + sharees | Archive readability / share rules |
| Vault passphrase not stored; vault not in Meilisearch | Vault + search indexing rules |
| Backups: “limited period” only — **no numeric retention published yet** | Backup config; refine when ops settles a number |

**Not claimed today (future hosting):** Cloud Run request logs recording IPs with a stated retention. Homelab/Compose-style managed hosting does not need that wording until a move that introduces those logs. Ops notes already flag adding Google Cloud + log retention if/when that happens.

---

## 5. Product context counsel should know

**What Linkdean is.** A bookmark manager: save links, folders, tags, notes, lists, dashboards; optional page archives (HTML/PDF/file); optional client-side **vault** encryption; sharing and public lists; browser extension; REST API with scoped API keys; MCP (read plus scoped bookmark writes) and downloadable Agent Skills that export **library** (non-vault) bookmarks as agent context.

**Commercial status.** Public beta. **No paid plans yet.** Roadmap next: define free vs paid tiers, then quotas, then checkout. Re-engage this brief before charging if counsel has not already signed off.

**Managed vs self-host.** Open-source app. Self-hosters configure their own legal URLs/version or none. These pages are for **linkdean.app** / `app.linkdean.app`.

**Data that can leave the user’s browser**

- Normal account/content traffic to the instance (and Cloudflare in front).
- Emails (verification, reset, invites) via iCloud Mail.
- Optional GoatCounter beacons (route-level; honour DNT/GPC).
- Extension: only on explicit save / folder-tag fetch / hash download for “color icon”, not full browsing history.
- Agent Skills / MCP: user-initiated; vault excluded from skill payloads; share/visibility rules apply.

**Abuse.** Signed-in Report on public lists, others’ profiles/comments; reasons include spam, harassment, illegal, impersonation, other. Admins set an abuse contact email; Terms/Privacy also list `abuse@linkdean.app`.

**Terms acceptance.** Password sign-up and first-admin need matching `terms_version`; SSO start can pass it; stored on user. When `TERMS_VERSION` changes, every older account sees accept-or-sign-out. API keys are **not** blocked by stale terms.

---

## 6. Operator obligations outside the pages

- **ICO data protection fee** (UK): individual with no staff — self-assessment treats the individual as staff for the band; confirm current Tier 1 fee on the ICO checker. Register address can be a PO box / virtual office (register is public). Appears on the register within about 7 working days of payment.
- Keep `VERSION` and `TERMS_VERSION` in lockstep when material Terms text changes.
- After paid plans exist, expect a follow-up review of pricing, cancellation, and any consumer-law wording.

---

## 7. How we ship counsel’s feedback

1. Agree redlines on Terms and/or Privacy (infra: `terms.astro` / `privacy.astro`).
2. Eng verifies every new factual claim against code (and updates the claims audit in the legal-pages skill reference if needed).
3. Bump `VERSION` in `Legal.astro` and `TERMS_VERSION` in managed compose to one new date.
4. Merge site PR (pages go live on merge); deploy guest so the app prompts re-accept.
5. Note the reviewed version in product features/changelog as needed.

This brief is the durable prep artifact until reviewed text ships.
