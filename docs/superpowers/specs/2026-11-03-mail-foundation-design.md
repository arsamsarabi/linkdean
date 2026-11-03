# Outbound email, part A: mail foundation

Roadmap item 1, **Outbound email**, splits into five sub-projects. Each gets its own spec, plan and build:

| # | Sub-project | Depends on |
| --- | --- | --- |
| A | Mail foundation (this spec) | none |
| B | Password reset by email | A |
| C | Email verification at sign-up and on email change (email change is new), invites to unverified addresses wait as pending invites | A |
| D | OIDC account link, so a local account with someone's email cannot block their first OIDC sign-in | C |
| E | Invite and notification emails, with per-user preferences | A |

## Decisions that apply to all five

- **Email is optional.** Without SMTP configured, the instance behaves as it does today: addresses count as trusted, there is no verification step, password reset is admin-only and nothing is emailed. B to E each degrade to today's behaviour when mail is off.
- **Configuration is environment variables only**, like every other setting. No SMTP secrets in the database.
- **Templates are MJML**, rendered in Rust by `mrml`, with variables filled by `minijinja`, sent by `lettre`. No Node in the mail pipeline.

## Operator

Environment variables:

| Variable | Required | Notes |
| --- | --- | --- |
| `SMTP_HOST` | to turn mail on | Mail is on when `SMTP_HOST` and `MAIL_FROM` are both set |
| `SMTP_PORT` | | Default 587, or 465 with `SMTP_TLS=tls` |
| `SMTP_USERNAME` / `SMTP_PASSWORD` | | Both or neither |
| `SMTP_TLS` | | `starttls` (default), `tls`, or `none` (refused unless `SMTP_HOST` is loopback, a private address, or a single-label name such as the Docker service `mailpit`; a single-label name could resolve through a DNS search domain, so use `none` only on networks you control) |
| `MAIL_FROM` | to turn mail on | Mailbox, e.g. `Linkdean <no-reply@example.com>` |
| `APP_URL` | existing | Base for every link in an email |

- The API refuses to start on invalid mail configuration (unparseable `MAIL_FROM`, only one of `SMTP_HOST` / `MAIL_FROM` set, only one of username / password, unknown `SMTP_TLS`), with a message naming the variable.
- An unreachable mail server does not block startup. It shows up in the Admin panel.
- Production compose passes every variable through. `docs/SELF_HOSTED.md` gains an Email section with examples for Gmail (app password), Fastmail and Postmark, and README's configuration table lists the variables.

## Delivery

- Sending never slows down or fails the request that caused it. Code queues an email; a background sender delivers it.
- Queue: an `email_outbox` table (recipient, template, rendered subject / HTML / text, status `queued` / `sent` / `failed`, attempts, next attempt time, last error, timestamps). Rendering happens at enqueue time, so a later template change never alters a queued email and the sender only talks SMTP.
- The sender runs in the API process, polls every few seconds, claims due rows with `FOR UPDATE SKIP LOCKED`, and retries with exponential backoff (30 s doubling, capped at 1 h) for up to 24 hours, then marks the row `failed`.
- Retention: sent rows are deleted after 7 days and failed rows after 30, by the existing hourly maintenance task. Rows hold the rendered body, which may contain one-time links, so they are not kept long.
- With mail off, enqueueing is a no-op that returns "mail disabled" to callers that need to know (B and C use this to fall back).

## Admin panel

Settings → Admin → Email:

- Status: Off, or On with host, port, TLS mode and sender. Never the password.
- Last 24 hours: sent, queued and failed counts, and the most recent error text with its time.
- **Send test email** to the signed-in admin's address. It sends directly rather than through the queue and shows success or the SMTP error inline. Rate limited.
- With mail off, the panel explains which variables turn it on and links to the docs.

## Developer

- The dev compose stack gains Mailpit. The dev API points at it (`SMTP_HOST=mailpit`, `SMTP_PORT=1025`, `SMTP_TLS=none`), so every dev email is caught and nothing leaves the machine. Its web UI is on `127.0.0.1:8025`.
- A new `crates/mail` crate owns rendering and SMTP sending; the outbox queries live in `crates/db` with the other repositories. Templates live in `crates/mail/templates/`:
  - `layout.mjml`: logo, wordmark, colours, footer. Every email extends it.
  - One `<name>.mjml` per email, with `<name>.subject` for the subject line.
  - `<name>.sample.json`: sample data for previews and tests.
- Templates are embedded in the binary at compile time. A typo fails tests, not production.
- Dev-only page `/api/dev/emails` lists every template, and `/api/dev/emails/<name>` renders it with its sample data. The route exists only when `ALLOW_DEFAULT_SECRETS=true` (dev). Edit, then reload (the dev API rebuilds on change).
- Plain text is generated from the rendered HTML, so each template is written once.
- Tests render every template with its sample data and fail on MJML errors, unknown variables (`minijinja` strict mode) or missing samples.
- Part A ships one template, `test` (used by the Admin test send), to exercise the whole path. B to E add theirs.

## Look and feel

- Light layout, brand header (text wordmark; no logo image, since many clients block remote images and a dev `APP_URL` is unreachable from them), violet primary button using the theme's filled violet, Satoshi with a system fallback stack.
- One clear action per email. The link is also printed as text under the button for clients that block buttons.
- Footer: instance name and `APP_URL`, a line saying why the recipient got the email, and for notification emails a link to email preferences (added in E).
- No dark-mode-specific styling. The light output stays readable in clients that darken emails.
- No tracking pixels or link rewriting.

## Testing

- Unit: config parsing and refusals; template rendering for every template; plain-text generation.
- Database (throwaway DB): enqueue, claim with `SKIP LOCKED` by two senders at once (each row sent once), backoff scheduling, give-up after 24 hours, retention.
- Integration: the sender delivers to a real SMTP server (Mailpit in a throwaway container) and the message arrives with HTML and text parts and the right `From`.
- Browser: the Admin Email panel in both states, and the test send arriving in Mailpit.

## Out of scope for A

Password reset, verification, email change, account linking, invite and notification emails, per-user email preferences, bounce handling, and DKIM/SPF set-up (the operator's mail provider handles these; the docs say so).
