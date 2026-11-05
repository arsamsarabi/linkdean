# Sign-up protection

Approved design for roadmap item **Sign-up protection**. Public beta uses **open registration**. Misuse is limited with Turnstile, a host-forced verified-email gate, and optional disposable-domain blocking. Invite-only remains an admin toggle, not the beta default.

## Goals

- Keep self-hosted instances unchanged unless the operator opts in.
- Make open registration on a public instance safe enough for a free beta.
- Reuse the existing email-verification hard gate and registration invite machinery.

## Cloudflare Turnstile

Off unless both env vars are set:

| Variable | Role |
| --- | --- |
| `TURNSTILE_SITE_KEY` | Public; returned in bootstrap / auth-config for the SPA |
| `TURNSTILE_SECRET_KEY` | Server only; used for Siteverify |

If exactly one is set, the API refuses to start.

On password `POST /api/auth/register`, when Turnstile is configured:

1. Require `turnstile_token` in the JSON body.
2. `POST https://challenges.cloudflare.com/turnstile/v0/siteverify` with `secret`, `response`, and `remoteip` (client IP).
3. Refuse with `400` when Cloudflare is unreachable, returns `success: false`, or the token is missing.

OIDC sign-in is not gated by Turnstile. Self-host with both keys unset keeps today's register path.

## Verified email (host force)

Existing platform setting `require_verified_email` and `email_unverified` AuthUser gate stay.

New env `REQUIRE_VERIFIED_EMAIL` (default off). When true **and** mail is configured, the gate applies even if the DB flag is false (logical OR). The admin UI shows the toggle on and disabled, with copy that the host set it.

Public beta Compose sets `REQUIRE_VERIFIED_EMAIL=true` together with SMTP.

## Disposable email domains

Env `BLOCK_DISPOSABLE_EMAILS` (default off). When true, registration and email-change reject addresses whose domain is in an embedded denylist (lowercase domain after `@`). Response: `400` with "Use a lasting email address."

No live download of the list in v1. Public beta sets the flag on.

## Out of scope

- Captcha on login
- Changing the default of `registration_open`
- Maintaining a community disposable-list feed
