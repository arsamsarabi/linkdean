# Plans and pricing

Defines Free vs paid tiers for the **managed** Linkdean service. Self-hosted instances stay outside plan gates: the operator sets [platform quotas](../self-hosting/operations.md#per-user-quotas); there is no Linkdean billing on self-host.

Organization seat plans (Free org size, Team, Business, Enterprise) live in [organizations and teams](./organizations-and-teams.md#saas-plans). This doc freezes the **personal** meters those plans sit on, including archive storage.

## Goals

- One place to read Free vs Pro personal limits before wiring `ResolvedLimits`.
- Keep self-host unlimited-by-default behaviour unless the operator sets caps.
- Reuse today's meters (`max_bookmarks`, `max_stored_image_bytes`, `max_enrich_actions_per_day`, `max_archive_bytes`, plus import/upload clamps). Do not invent parallel counters.
- Align archive Free with the hosted beta already documented at 500 MB ([archive design](./archive.md#quota-and-limits)).

## Non-goals

- Inventing dollar amounts in this repo (prices live in the Stripe Dashboard / `STRIPE_PRICE_ID_PRO`).
- Trials, coupons, and tax details (configure in Stripe; counsel before charging — see [legal review](./legal-review.md)).
- Org seat checkout (Team/Business) — personal Pro only until Orgs billing.
- Changing self-host licence text beyond "free for non-commercial use" already on the roadmap.
- Per-seat archive multipliers for Team/Business (decide with Orgs Phase A / Archive Phase C).

## Personal plans (managed)

| Meter | Free | Pro |
| --- | --- | --- |
| Owned bookmarks (not deleted) | 5 000 | Unlimited |
| Stored image bytes (avatar + preview/favicon/OG) | 1 GiB | 10 GiB |
| Enrich actions per UTC day | 500 | 5 000 |
| Archive storage (`max_archive_bytes`) | 500 MiB | 25 GiB |
| Largest archive copy | Platform item cap (25 MiB default) | Same |
| Import / upload image size | Hard caps only (today) | Same |
| Vaults | 1 | Unlimited |
| Organizations | As Free row in [orgs SaaS table](./organizations-and-teams.md#saas-plans) | Same Free org row until a Team seat is purchased |

Notes:

- **Unlimited** means no plan ceiling; a platform-wide emergency cap may still apply in ops incidents (operator-only, not a product plan).
- Guests, Team/Business/Enterprise feature packs, and seat rules stay in the orgs design. A personal Pro subscription does not by itself unlock Team features.
- Downgrade: never lock the person out of existing data. Over-limit bookmarks, archives and vaults stay readable/exportable; creating or enlarging past the Free caps is blocked (same rule as orgs for vaults).

## How limits combine (for Phase B)

Today `ResolvedLimits` is a copy of the platform row (`None` = unlimited). After plans:

1. Resolve the account's **plan limits** from the table above (and org plan when in a team workspace — Orgs work).
2. Resolve **platform** limits as today.
3. Effective limit per meter = the tighter of the two when both are set; if either side is unlimited (`None`), use the other; if both unlimited, unlimited.

Plan checks live in that one resolver path so SaaS gating does not scatter through handlers ([orgs note](./organizations-and-teams.md#saas-plans)).

## Self-host

- No Free/Pro assignment. Quotas are only what the admin sets under **Admin → Quotas**.
- Commercial use of self-host remains a product/legal follow-up; this design does not add runtime licence checks.

## Phase map

| Phase | Done when |
| --- | --- |
| **A — Define tiers** (this doc) | Done — Free vs Pro personal meters + archive numbers; org seat matrix linked. |
| **B — Enforce quotas** | Done — `MANAGED_SERVICE` assigns Free; resolver merges plan + platform; Settings shows plan and caps; admin can set Free/Pro. |
| **C — Checkout** | Done — Stripe Checkout + Customer Portal for personal Pro; webhooks set `users.plan`; price id from env (no dollar amount in repo). |

## Checkout (Phase C)

Managed only (`MANAGED_SERVICE=true`) and only when all of `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and `STRIPE_PRICE_ID_PRO` are set:

1. Settings → **Upgrade to Pro** → `POST /api/billing/checkout` → Stripe Checkout (subscription mode).
2. Stripe webhooks (`POST /api/billing/webhook`) on `checkout.session.completed` / subscription updated/deleted → `set_user_plan` (+ store `users.stripe_customer_id`).
3. **Manage billing** → `POST /api/billing/portal` → Stripe Customer Portal (invoices, payment method, cancel).
4. Admins may still set Free/Pro for comps/support; the next webhook for that customer can overwrite the plan.

Self-host and managed without Stripe keys: no billing routes or Settings CTAs.

## Still open (not blocking C)

- Whether managed Free also caps `max_import_bytes` / `max_upload_image_bytes` below the hard caps.
- Team archive pool: flat per workspace vs per seat × Pro archive.
- Whether “Unlimited” bookmarks on Pro gets a soft abuse ceiling in ops tooling only.
- Live dollar price, tax, and paid-plan Terms (counsel); set the Price in Stripe, not here.
