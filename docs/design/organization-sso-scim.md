# Organization SSO and SCIM

Product design for Organizations **Phase D**. Complements [organizations and teams](./organizations-and-teams.md). Instance-wide OIDC/GitHub/Google/Microsoft SSO already ships; this adds **per-organization** identity.

## Goals

- An organization admin can connect one OpenID Connect IdP so members sign in through that IdP and land in the organization.
- An organization admin can issue a SCIM bearer token so the IdP provisions and deprovisions members.
- Self-host has the feature with no plan gate. Managed SaaS will gate on Enterprise later (same deferred pattern as invite links / verified domains).

## Non-goals (this phase)

- SAML 2.0 — deferred; roadmap keeps “SAML or per-organization OIDC”; ship OIDC first (reuse existing `openidconnect` stack).
- SCIM Groups / team mapping — Users only; team assignment stays invites / default-team join.
- Multiple IdPs per organization.
- Forcing SSO-only (disabling password) for an organization — optional later policy.
- Replacing instance-level SSO env providers.

## Decisions

| Topic | Choice |
| --- | --- |
| Protocol | Per-org **OIDC** authorization code + PKCE + nonce (same security model as instance OIDC). |
| IdPs per org | At most **one** enabled config. |
| Domain binding | Org SSO may be **enabled** only when the org has ≥1 **verified** domain. The IdP email’s domain must match a verified domain of that org. |
| Default membership | New joiners get org role **member** and default-team **editor** (same as domain auto-join). |
| Account identity | Still one `users.oidc_issuer` + `oidc_subject` per account. Org SSO uses the org IdP’s issuer URL. If the account already has a different issuer, sign-in fails with a clear error until they disconnect instance SSO (or use a different account). |
| Client secret at rest | AES-GCM ciphertext + nonce, key derived from `SESSION_SECRET` (label `linkdean-org-sso-v1`). Rotating `SESSION_SECRET` voids stored secrets — document beside unsubscribe links. |
| Discovery | Public `POST /api/auth/org-sso/discover` with `{ "email" }`. When the domain matches a verified domain on an org with SSO enabled, return `{ organization_id, organization_name, sso: true }`. Otherwise `{ sso: false }` with the same shape (no org leak for unknown domains). |
| Login paths | `GET /api/auth/org-oidc/start?organization_id=` → IdP; `GET /api/auth/org-oidc/callback` shared callback. Flow cookie scoped to `/api/auth/org-oidc`. |
| JIT vs SCIM | OIDC callback can create the user and join the org (JIT). SCIM can create/update/deactivate independently; both paths converge on membership + optional OIDC subject when present. |
| SCIM surface | `/api/scim/v2/Users` (+ `ServiceProviderConfig`, `Schemas`, `ResourceTypes` stubs enough for Entra/Okta). Bearer token `ldscim_`… hashed at rest (same pattern as `ldk_` API keys). No Groups. |
| SCIM deactivate | `active: false` (PATCH/PUT) or DELETE → remove org membership, end sessions; do **not** hard-delete the user account (personal library stays). |
| Plan gate | None in this phase. |
| Audit | Record `sso.configured`, `sso.enabled`, `sso.disabled`, `sso.login`, `scim.token_created`, `scim.token_revoked`, `scim.user_provisioned`, `scim.user_updated`, `scim.user_deprovisioned`. |

## Data model

### `organization_sso_configs`

One row per organization (unique `organization_id`).

| Column | Notes |
| --- | --- |
| `issuer_url` | OIDC issuer (discovery). |
| `client_id` | Public. |
| `client_secret_ciphertext` / `client_secret_nonce` | Encrypted. |
| `display_name` | Button label (default “Company SSO”). |
| `enabled` | When true and ≥1 verified domain, discovery and start work. |
| `created_by_user_id`, timestamps | |

### `organization_scim_tokens`

| Column | Notes |
| --- | --- |
| `organization_id` | |
| `token_hash`, `prefix` | Like API keys. |
| `name`, `created_by_user_id`, `last_used_at`, `revoked_at`, timestamps | |

At most one **active** token per org (creating a new one revokes the previous). Plaintext shown once at create.

## API (admin, session)

- `GET/PUT/DELETE /api/organizations/{id}/sso` — owner/admin. PUT upserts config (secret optional on update = keep existing). Response never returns the secret.
- `POST /api/organizations/{id}/sso/test` — optional later; not required for v1 if discovery + live login cover it.
- `GET/POST /api/organizations/{id}/scim-tokens` — list (prefix only) / create (returns plaintext once).
- `DELETE /api/organizations/{id}/scim-tokens/{token_id}` — revoke.

## API (auth / SCIM)

- Discover + org-oidc start/callback as above.
- SCIM under `/api/scim/v2/...` authenticated by SCIM bearer (not session, not `ldk_`). Map `userName` / primary email to Linkdean email; require verified-domain match.

## UI

- Organization settings → **SSO** tab: issuer, client id/secret, display name, enable toggle (disabled until a verified domain exists), redirect URI copy hint (`{APP_URL}/api/auth/org-oidc/callback`).
- Same tab → **SCIM**: create/revoke token, base URL copy hint.
- Login page: email field can offer “Continue with company SSO” after discover returns `sso: true`.

## Delivery slices

1. ~~**D1 — OIDC backend**~~ — shipped.
2. ~~**D2 — SSO settings + login UI**~~ — shipped.
3. ~~**D3 — SCIM**~~ — shipped (Users + tokens; Groups deferred).

Merge in order; stack D2/D3 on the prior branch while earlier PRs are open.

## Open follow-ups (not blocking)

- SAML.
- SSO-only org policy.
- SCIM Groups → teams.
- Enterprise plan gate when org seat checkout exists.
