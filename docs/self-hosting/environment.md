# Environment variables

Everything goes in `.env`. Compose reads it for the `${...}` values in `docker-compose.yml`, and `env_file: .env` hands it to the API. Values with spaces need quotes: `MAIL_FROM="Linkdean <no-reply@example.com>"`.

An unset variable takes its default. An empty one (`SMTP_HOST=`) counts as unset.

Published copy: [docs.linkdean.app](https://docs.linkdean.app/docs/self-hosting/environment). Install steps: [quick-start.md](./quick-start.md). Operations: [operations.md](./operations.md).

### Required

The stack does not start without these. Generate each with `openssl rand -hex 32`: hex stays valid inside the database URL. The API refuses the example values published in this repository.

| Variable | What it does |
| --- | --- |
| `POSTGRES_PASSWORD` | Password of the `linkdean` database user. Postgres applies it only when the `pgdata` volume is first created; changing it later needs `ALTER USER`, see [Upgrading an existing install](./operations.md#upgrading-an-existing-install) |
| `MEILI_MASTER_KEY` | Meilisearch master key. The search container runs with it, and the API uses it to index and search. At least 16 bytes |
| `SESSION_SECRET` | At least 16 characters. Checked at startup and reserved for signing; sessions today are random tokens stored hashed in Postgres |

### Recommended

| Variable | Default | What it does |
| --- | --- | --- |
| `LINKDEAN_VERSION` | `latest` | Image tag to run. Pin a release such as `0.0.1` so upgrades happen only when you change it |
| `APP_URL` | `http://localhost:8080` | Public address of the instance, with no trailing slash. Used for links in emails, the SSO callback and the allowed browser origin. An `https://` value also marks the session cookie `Secure`. Set it whenever the instance is reached by any other address |
| `TRUST_FORWARDED_FOR` | `false` | Set `true` when a reverse proxy (Caddy, nginx, Traefik, a tunnel) is the only route to the API. Login and registration limits then count each client by the last `X-Forwarded-For` entry. Unset behind a proxy, every user shares the proxy's address and one person's failed logins can lock everyone out. Leave it unset when the API is reachable directly, because clients could forge the header |
| `HSTS_MAX_AGE` | unset | When set to a number of seconds, every response includes `Strict-Transport-Security: max-age=<n>; includeSubDomains`. Leave unset on plain HTTP (including local Docker) so browsers are not stuck requiring HTTPS. On `linkdean.app` behind Cloudflare Tunnel you can also set HSTS at the edge instead |
| `BOOTSTRAP_TOKEN` | unset | When set, the first account (the admin) must enter it on the setup page and must use a password, not SSO. Unset, whoever registers first owns the instance. Ignored once any account exists |

`true`, `1`, `yes` and `on` all switch a true/false setting on. Anything else is off.

### Email

Off unless both `SMTP_HOST` and `MAIL_FROM` are set. Without it the instance sends nothing. Provider examples and the admin test page: [Email](./operations.md#email).

| Variable | Default | What it does |
| --- | --- | --- |
| `SMTP_HOST` | unset | SMTP server host name |
| `MAIL_FROM` | unset | Sender mailbox, such as `"Linkdean <no-reply@example.com>"` |
| `SMTP_TLS` | `starttls` | `starttls`, `tls` (implicit TLS) or `none`. `none` sends mail unencrypted and is refused unless the host is `localhost`, a loopback or private address, or a single-label name such as `mailpit`. Use it only on a network you control |
| `SMTP_PORT` | `587`, or `465` with `SMTP_TLS=tls` | SMTP port |
| `SMTP_USERNAME` | unset | SMTP login. Set together with `SMTP_PASSWORD`, or leave both unset |
| `SMTP_PASSWORD` | unset | SMTP password, used exactly as given |

The API refuses to start when only one of a pair is set, `SMTP_TLS` or `SMTP_PORT` is invalid, or `MAIL_FROM` is not a valid mailbox.

### Sign-up protection

Off by default. Public beta hosts usually set all three with SMTP.

| Variable | Default | What it does |
| --- | --- | --- |
| `TURNSTILE_SITE_KEY` | unset | Cloudflare Turnstile site key. With `TURNSTILE_SECRET_KEY`, password registration shows a captcha and the API checks the token with Siteverify. OIDC sign-in is unchanged |
| `TURNSTILE_SECRET_KEY` | unset | Turnstile secret. Set together with `TURNSTILE_SITE_KEY`, or leave both unset |
| `REQUIRE_VERIFIED_EMAIL` | `false` | When `true` and mail is on, the verified-email hard gate applies even if the admin toggle is off. The admin UI shows the toggle on and disabled |
| `BLOCK_DISPOSABLE_EMAILS` | `false` | When `true`, registration and email-change refuse addresses on an embedded disposable-domain list (`400` "Use a lasting email address.") |

The API refuses to start when only one Turnstile key is set.

### Single sign-on (OIDC)

On when all three are set. Details and how SSO meets existing password accounts: [Single sign-on](./operations.md#single-sign-on).

| Variable | What it does |
| --- | --- |
| `OIDC_ISSUER` | Issuer exactly as the provider's `/.well-known/openid-configuration` reports it, trailing slash included. Authentik: `https://auth.example.com/application/o/<slug>/` |
| `OIDC_CLIENT_ID` | Client ID from the provider |
| `OIDC_CLIENT_SECRET` | Client secret from the provider |

Register `{APP_URL}/api/auth/oidc/callback` as the redirect URI. A new identity gets an account only while registration is open.

### Screenshots

| Variable | Default | What it does |
| --- | --- | --- |
| `BROWSER_WS_URL` | unset | Chrome DevTools address, `http://chrome:9222` with the service above. Unset, no screenshots are taken |
| `BROWSER_PROXY_BIND` | unset | Address the API serves Chrome's egress proxy on, `0.0.0.0:3128` with the service above. Only starts when `BROWSER_WS_URL` is set too. The proxy connects only to public addresses, and only to ports 80, 443 and 1024 and up. The API refuses to start if the value is not `address:port` |

### Logging

| Variable | Default | What it does |
| --- | --- | --- |
| `LOG_FORMAT` | `text` | `text` for human stdout, `json` for one JSON object per line on stdout. Invalid values stop startup |
| `LOG_FILE` | unset | When set, append JSON lines (JSONL) to this path even if stdout stays `text`. Create the file if missing; open failures stop startup. Rotate the file yourself (for example logrotate); the process does not |
| `RUST_LOG` | `info` | Log filter, such as `debug` or `info,linkdean_api=debug` (also listed under Advanced) |

Every HTTP response includes `X-Request-Id`. A valid inbound value (1–128 characters: letters, digits, `_`, `-`, `.`) is kept; otherwise the API mints a UUID. The same id appears on the request's log span.

### Metrics

| Variable | Default | What it does |
| --- | --- | --- |
| `METRICS_TOKEN` | unset | Turns on `GET /api/metrics` (Prometheus format), which then needs `Authorization: Bearer <token>`. Unset, the route returns `404` |

### Broken-link checker

A background worker checks saved links and flags dead ones under **Library health**. Vault bookmarks and private hosts are skipped.

| Variable | Default | What it does |
| --- | --- | --- |
| `LINK_CHECK_ENABLED` | `true` | Set `false` to turn the checker off |
| `LINK_CHECK_TICK_SECS` | `60` | Seconds between rounds |
| `LINK_CHECK_BATCH_SIZE` | `40` | Links checked per round |
| `LINK_CHECK_CONCURRENCY` | `2` | Links checked at once |
| `LINK_CHECK_PER_HOST_DELAY_MS` | `1000` | Pause between two requests to the same host |
| `LINK_CHECK_RECHECK_DAYS` | `7` | Days before a checked link is checked again |

### Advanced

These have working defaults inside the image. Change them only for a reason.

| Variable | Default | What it does |
| --- | --- | --- |
| `SESSION_DAYS` | `30` | Days a sign-in lasts |
| `CORS_ORIGINS` | `APP_URL` | Comma-separated browser origins allowed to call the API. `*` allows any origin; avoid it on a reachable server |
| `RUST_LOG` | `info` | Log filter, such as `debug` or `info,linkdean_api=debug` |
| `BIND` | `0.0.0.0:8080` | Listen address inside the container. If you change the port, change `ports:` and the healthcheck too |
| `DATABASE_URL` | set by the compose file | Postgres connection string. Set it in `.env` only to use your own Postgres server, and remove `DATABASE_URL` from the compose file's `environment:` so yours is used |
| `MEILI_URL` | set by the compose file | Meilisearch address |
| `AVATAR_DIR` | `/app/data/avatars` | Where uploaded avatars are stored. Keep it on the `avatars` volume |
| `PREVIEW_DIR` | `/app/data/previews` | Where screenshots and cached favicons and Open Graph images are stored. Keep it on the `previews` volume |
| `STATIC_DIR` | `/app/static` | Built web app served by the API |

### Never in production

| Variable | What it does |
| --- | --- |
| `ALLOW_DEFAULT_SECRETS` | Lets the API start on the secrets published in this repository and turns on development-only routes. For local development only |
