# Security Policy

## Supported versions

Linkdean is pre-1.0 and moves quickly. Security fixes land in the next release of the Docker image, `ghcr.io/arsamsarabi/linkdean`, and on `main`.

| Version | Supported |
| --- | --- |
| Latest release (`latest` and the newest `X.Y.Z` tag) | Yes |
| `main` | Yes |
| Older 0.x releases | No |

If you can't upgrade right away, still report the issue. I can suggest a workaround when one exists.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Email **arsamsarabi@me.com** with:

- A short description of the issue
- The affected version (image tag, `LINKDEAN_VERSION`, or commit)
- How the instance was set up, if it matters (published image or a build from source, behind a reverse proxy or not, and which optional features are on: SMTP, OIDC single sign-on, Chrome screenshots)
- Steps to reproduce, or a proof of concept if you have one
- The impact: what an attacker can read, change or take over, and what access they need first

You should get an acknowledgment within **a few business days**. After that I'll keep you updated while I triage and, if needed, prepare a fix.

### What to expect

- **Accepted:** I'll work on a fix, credit you in the release notes if you'd like, and coordinate disclosure once a patched image is published (or after a window we agree on).
- **Declined:** I'll explain why: for example, not reproducible, out of scope, or behavior the documentation already describes.

## Scope

Linkdean is a multi-user web app that stores people's bookmarks, private vaults and shares, and fetches pages they save. Reports about any of these are in scope:

- Signing in as someone else, or bypassing login, sessions, password reset, email verification, OIDC account linking or the first-admin `BOOTSTRAP_TOKEN`
- Reading or changing another user's bookmarks, folders, vaults, shares, profile or API keys, or acting as an admin without being one
- Weaknesses in vault encryption, passphrases or recovery keys that expose vault contents
- Cross-site scripting, cross-site request forgery, or stored `javascript:`/`data:` URLs that run in another user's browser
- SQL injection or other injection into the database, search index or outgoing email
- Server-side request forgery: making the preview fetcher, link checker or Chrome screenshot service reach private or internal addresses
- Escaping the Chrome container's sandbox or reaching the database or search network from it
- Getting around the documented login, registration or unlock limits
- Leaking secrets, session tokens, API keys or other users' email addresses through responses, logs, emails or exports
- Supply-chain problems in what I publish: the Docker image, the release workflow or the CI configuration

Out of scope (report these as normal bugs unless you see a security angle I've missed):

- Deployments that go against [docs/self-hosting/quick-start.md](../docs/self-hosting/quick-start.md) or [docs/self-hosting/operations.md](../docs/self-hosting/operations.md): for example, `ALLOW_DEFAULT_SECRETS` in production, `TRUST_FORWARDED_FOR=true` without a reverse proxy in front, or publishing Postgres or Meilisearch ports
- Behavior the docs already describe as a known limitation, such as the account lockout that anyone who knows an email can trigger, or rate-limit counters that reset when the API restarts
- What an instance admin can do by design
- Attacks that need an already compromised host, browser or account
- Denial of service through sheer request volume
- Vulnerabilities only in upstream software (Postgres, Meilisearch, Chrome, Docker): please report those to their projects, and tell me too if Linkdean's configuration makes them worse

## Prefer GitHub's private reporting?

If the repository has [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) turned on, you can use **Security → Report a vulnerability** instead of email. Either channel is fine.
