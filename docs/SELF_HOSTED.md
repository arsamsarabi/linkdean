# Self-host Linkdean

This page covers the production stack in `deploy/docker/docker-compose.yml`. For local development, see [Development setup](#development-setup) below and the README.

## Quick start

```bash
git clone https://github.com/arsamsarabi/linkdean.git
cd linkdean
printf 'POSTGRES_PASSWORD=%s\nMEILI_MASTER_KEY=%s\nSESSION_SECRET=%s\n' \
  "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > deploy/docker/.env

docker compose --env-file deploy/docker/.env -f deploy/docker/docker-compose.yml up -d --build
```

Open `http://localhost:8080`. The first registered user becomes admin. Set `BOOTSTRAP_TOKEN` (see [First account](#first-account)) when the server is reachable by others before you register.

1. Create `deploy/docker/.env` with the [required secrets](#required-secrets) and set `APP_URL`. Behind a reverse proxy, also set `TRUST_FORWARDED_FOR=true` (see [Reverse proxy](#reverse-proxy)).
2. Compose up (command above).
3. On first visit, register (bootstrap admin).
4. Optional OIDC: set issuer, client id, and secret. `OIDC_ISSUER` is the provider's issuer exactly as its `/.well-known/openid-configuration` reports it, trailing slash included (Authentik: `https://auth.example.com/application/o/<slug>/`). Callback: `{APP_URL}/api/auth/oidc/callback`. OIDC sign-in obeys closed registration: an unknown identity gets no account unless registration is open. See [Single sign-on](#single-sign-on) for how it meets existing password accounts.
5. Avatars persist in the `avatars` volume (`AVATAR_DIR`); screenshots + favicon/OG binaries in `previews` (`PREVIEW_DIR`). Include both with Postgres backups.

Secrets are injected via environment variables (or the operator's preferred secret store). No Infisical dependency. `deploy/docker/.env` is gitignored; keep it out of version control.

## Production deployment

### Required secrets

The production Compose file refuses to start unless all three are set:

| Variable | Used by | Notes |
| --- | --- | --- |
| `POSTGRES_PASSWORD` | `db`, and `api` through `DATABASE_URL` | Use hex (`openssl rand -hex 32`) so it stays valid inside the connection URL |
| `MEILI_MASTER_KEY` | `meilisearch`, `api` | Meilisearch runs with this key; the API uses it to index and search |
| `SESSION_SECRET` | `api` | Long random string |

Without one of them, `docker compose up` stops with `set POSTGRES_PASSWORD` (or the matching name).

The API also refuses to start when a secret is a value published in this repository: the `.env.example` and dev Compose defaults for `SESSION_SECRET` and `MEILI_MASTER_KEY`, and the `linkdean:linkdean` database login. Generate your own values. `ALLOW_DEFAULT_SECRETS=true` turns this check off; it exists for local development only and must never be set in production.

### First account

The first account on an empty instance becomes admin. Sign-ups are serialised, so two people registering at once cannot both become admin, but without a token whoever registers first owns the instance.

Set `BOOTSTRAP_TOKEN` to a long random string (`openssl rand -hex 32`) to require it for that first account. The setup page asks for it. With a token set, the first account must register with a password: an OIDC sign-in cannot carry the token and is refused until an admin exists. Once any account exists the token is ignored and you can remove it.

### Reverse proxy

Login, registration and vault-unlock limits count attempts per client address. By default the API uses the address of the TCP connection.

- **Behind a reverse proxy** (Caddy, nginx, Traefik, a tunnel): set `TRUST_FORWARDED_FOR=true`. Otherwise every request appears to come from the proxy, all users share one counter, and a few failed logins from anyone block logins for everyone.
- **API reachable directly**: leave `TRUST_FORWARDED_FOR` unset. With it on, a client can forge `X-Forwarded-For` and pick any address, which defeats the per-address limits.

Only enable it when the proxy is the sole route to the API, and make sure the proxy appends the client address to `X-Forwarded-For`. The API reads the last entry, the one your proxy wrote. With a chain of proxies, the last entry is the nearest proxy's view, so terminate the chain at a single proxy in front of the API.

### Container runtime

- The API image runs as an unprivileged user, `linkdean` (UID/GID 10001). It owns `/app/data/avatars` and `/app/data/previews`; the binary and static files stay read-only to it.
- Every service uses `restart: unless-stopped`, so the stack comes back after a crash or host reboot.
- Only `api` publishes a port (`8080`). Postgres, Meilisearch and Chrome stay on the Compose network.
- Chrome (`chromedp/headless-shell`, pinned) takes page screenshots for bookmark previews, so it opens every page users save. It sits on its own `browser` network with the API and has no route to Postgres or Meilisearch. It runs as `nobody` with a read-only filesystem, no capabilities, `no-new-privileges`, 1 GB of memory and 512 processes, and with Chrome's own sandbox on.
- That sandbox needs user namespaces, which Docker's default seccomp profile blocks. `deploy/docker/chrome-seccomp.json` is Docker's default profile plus `clone`, `unshare`, `setns` and `chroot`, and must sit next to the compose file. Some hosts also restrict unprivileged user namespaces through AppArmor (Ubuntu 23.10 and later). There Chrome exits with `No usable sandbox!` in its logs, and bookmarks fall back to OG images and favicons. Allow it with `sysctl kernel.apparmor_restrict_unprivileged_userns=0`, or leave `BROWSER_WS_URL` unset to run without screenshots.
- Update the Chrome image tag with each stable Chrome release.

### Limits and behaviour operators should know

| Area | Behaviour |
| --- | --- |
| Failed logins | 10 per address per account, 50 per address, 50 per account, each per 15 minutes. Past a limit the API returns `429` until the window ends. A successful login clears only the per-address-per-account counter |
| Account lockout | 50 failed logins on one account from any address lock that account for 15 minutes, **including the correct password**. Someone who knows a user's email can trigger this on purpose |
| Registration | 10 attempts per address per 15 minutes, counted whether or not they succeed |
| Email verification and change | Emails: 3 per account and 10 per client IP. Link uses: 10 per client IP. Each per 15 minutes |
| Vault unlock, recovery, global unlock, recovery key replacement, global passphrase off | 10 failed attempts per user per 15 minutes, shared across all of these |
| Limit storage | Counters live in API memory. Restarting the API resets them, and several API replicas would each keep their own |
| Not limited yet | Password change and global vault passphrase change |
| Password hashing | Argon2id at 64 MiB and 3 passes per hash or vault key unwrap. At most 4 run at once (up to 256 MiB); extra logins queue. Older, cheaper password hashes are upgraded at the next sign-in |
| Passwords and vault passphrases | New ones need at least 12 characters. Existing shorter ones still work |
| Expired session | The API returns `401` with `{"code":"session_expired"}`, and the web app signs the user out and opens `/login`. A wrong password stays a plain `401` |
| Database queries | Each statement times out after 30 seconds. Migrations run without this limit |
| Import | Request body up to 20 MiB; file content up to 10 MiB; folders nested at most 64 deep; at most 10,000 folders per import. Folder loops are refused |
| Avatar upload | Request body up to 6 MiB (larger uploads currently fail with `400`, not `413`) |
| Preview upload | Request body up to 9 MiB |
| Image decode (avatar, preview) | At most 40 megapixels and 160 MiB of decode memory per image; two decodes run at once and the rest wait |
| Trash retention | User setting from 1 to 3,650 days, enforced by a database `CHECK` |

### Email

Mail is optional. Without it the instance runs as before and sends nothing. Set these on the `api` service (the production Compose file passes them through):

| Variable | Notes |
| --- | --- |
| `SMTP_HOST` | Mail server. Mail is on when this and `MAIL_FROM` are both set |
| `MAIL_FROM` | Sender, e.g. `Linkdean <no-reply@example.com>` |
| `SMTP_TLS` | `starttls` (default), `tls` (implicit TLS) or `none` |
| `SMTP_PORT` | Default `587`, or `465` with `SMTP_TLS=tls` |
| `SMTP_USERNAME` / `SMTP_PASSWORD` | Set both or neither. The password is used as given, spaces included |

The API refuses to start when:

- only one of `SMTP_HOST` and `MAIL_FROM` is set
- only one of `SMTP_USERNAME` and `SMTP_PASSWORD` is set
- `SMTP_TLS` is not `starttls`, `tls` or `none`, or `SMTP_PORT` is not a port number
- `MAIL_FROM` is not a valid mailbox
- `SMTP_TLS=none` is set for a host that is not local. `none` sends mail unencrypted, so it is allowed only for `localhost`, loopback and private addresses, and single-label names such as `mailpit`. A single-label name can still resolve through your DNS search domain, so use `none` only on a network you control

Examples:

- **Gmail:** `SMTP_HOST=smtp.gmail.com`, port `587`, STARTTLS (the default). Use an app password, which needs 2-Step Verification on the account. `MAIL_FROM` must be the Gmail address or a verified alias.
- **Fastmail:** `SMTP_HOST=smtp.fastmail.com`, `SMTP_TLS=tls` (port `465`), with an app password.
- **Postmark:** `SMTP_HOST=smtp.postmarkapp.com`, port `587`. `SMTP_USERNAME` and `SMTP_PASSWORD` are both the server API token, and the `MAIL_FROM` address needs a verified sender signature.

Check it works: **Settings → Admin → Email → Send test email**. The panel shows the settings in use (never the username or password), sent, queued and failed counts for the last 24 hours, and the latest error. The test goes straight to SMTP, skipping the queue, to the admin's own address, and shows the server's answer. Each admin can send 5 tests per 15 minutes.

Delivery:

- Email is queued in Postgres and sent in the background by the API. A failed send is retried after 30 seconds, then at doubling intervals up to an hour apart. An email still unsent a day after it was queued is marked failed.
- Each SMTP step times out after 20 seconds, and a whole send after 60.
- Sent emails are deleted after 7 days and failed ones after 30.
- Delivery is at least once: if the API stops after the server accepted an email but before it was marked sent, it is sent again.

SPF, DKIM and DMARC are set up with your mail provider and DNS, not in Linkdean.

Password reset:

- With mail on, the login page links to **Forgot password?**. The emailed link works once, for 30 minutes. Links are built from `APP_URL`, so set it to the address people open Linkdean at.
- With mail off, the login page tells people to ask an admin. An admin opens **Settings → Admin → Accounts**, picks the user and chooses **Reset link**. The link is shown once, works once, for 24 hours, and must be sent privately: anyone with it can set the password. With mail on, the user is also told by email that an admin made one.
- A new link voids the previous one, and so does a normal password change. A reset ends every session, signs in the browser that reset, and leaves every vault locked; vault passphrases do not change.
- Accounts that sign in only with OIDC have no password to reset. They get an email saying so, and the admin link is refused for them.
- Each email address can request 3 links per 15 minutes, and each client IP 10. Each client IP can try 10 links per 15 minutes.

Email verification:

- Verification needs mail. With mail on, a password registration sends a link to the new address, and unverified accounts see a banner with **Resend email**. The link works once, for 24 hours, and a new one voids the previous one. Opening it verifies the address without signing in.
- OIDC accounts count as verified when the identity provider reports the address as verified.
- A folder or dashboard share to an unverified account does not reach it. The invite waits for the address and attaches when the account verifies, after which the account still accepts it as usual.
- With mail off, new accounts stay unverified, there is no banner, and invites to them wait as pending invites. The invite's claim link still works, now also while signed in, when the signed-in account's address matches the invite. An admin can also mark the account verified.
- People change their own address under **Profile → Edit profile → Change email**, with their current password. The link goes to the new address and the old one gets a notice; the address only changes when the link is opened. Accounts that sign in only with OIDC cannot change it here. With mail off the option says to ask an admin.
- **Settings → Admin → Accounts** has **Mark verified**, **Remove verification** and **Change email** for each account. An admin email change applies at once and counts as verified. Removing verification keeps shares already accepted; only new invites wait.
- **Require verified email** under **Settings → Admin** holds unverified accounts at a verify screen with **Resend email** and **Sign out**, and the API answers `403` with `{"code":"email_unverified"}` on every route outside `/api/auth/`. It can only be turned on while mail is on, and it never applies to admins, so it cannot lock everyone out. If you turn mail off while it is on, the gate stops applying until mail is back.
- Each account can send 3 verification or email change emails per 15 minutes, and each client IP 10. Each client IP can try 10 verification or email change links per 15 minutes.
- **Upgrading:** every account that exists when you upgrade counts as verified (migration `042`). If you do not trust an account's address, remove its verification under **Settings → Admin → Accounts**.

### Single sign-on

Accounts are matched to the identity provider by issuer and subject, never by email alone. When an OIDC sign-in brings an address that a password account already uses:

- The login page says "An account already uses x•••@example.com. Sign in with its password to connect single sign-on." Signing in with that account's password attaches the provider identity to it and shows **Single sign-on connected**. From then on the provider alone signs in.
- The waiting link lives on the server for 10 minutes and is tied to the browser by an HttpOnly cookie (`linkdean_oidc_link`); nothing identifying goes in the URL. It is used up by the next password sign-in in that browser, whether or not it linked. Nothing links when the password is wrong, the link expired, or the password belongs to a different account than the one holding the provider's address. The usual login limits apply.
- The account's address gets a "Single sign-on was connected" email (when mail is on), so a link nobody expected can be undone by resetting the password and telling the admin.
- Linking marks the account verified only when the provider reports that same address as verified.
- If the person does not know the account's password, the page says to ask the admin. An admin can reset that account's password, or change its address under **Settings → Admin → Accounts → Change email**, which frees the address for a normal single sign-on.
- An account that signs in only with OIDC, or that is already connected to a different identity, is not linked: the login page says why.

Signed-in people manage it under **Profile → Edit profile → Single sign-on** (shown when OIDC is configured):

- **Connect** runs the provider sign-in and attaches that identity to the signed-in account, even when the provider's address differs. The account's address stays as it is.
- **Disconnect** needs the current password and is only offered to accounts that have one, so nobody loses their only way to sign in. Wrong passwords count against the login limits.
- One provider identity belongs to at most one account, and an account has at most one identity. Connecting an identity already attached elsewhere is refused: "This sign-in is already connected to another account."

### API clients

`PATCH` bodies distinguish a missing field from `null`:

- Field omitted → left unchanged.
- Field set to `null` → cleared. This applies to profile `display_name`, `bio` and `timezone`; bookmark `description`, `notes` and `category_id`; and category `parent_id` (moves the category to the root) and `icon`.

Scripts written against older builds that sent `null` to mean "no change" will now clear those fields.

### Upgrading an existing install

Earlier builds ran as root with a default database password. Before upgrading past commit `bdef6d8`:

1. **Database password.** Postgres sets its password only when the `db` volume is first created, so a new `POSTGRES_PASSWORD` does not change an existing database. If the database still uses `linkdean`, change it to match your new value before starting the new `api`:

   ```bash
   docker compose -f deploy/docker/docker-compose.yml exec db \
     psql -U linkdean -d linkdean -c "ALTER USER linkdean PASSWORD '<new POSTGRES_PASSWORD>'"
   ```

   The API refuses the old `linkdean:linkdean` login, so skipping this step stops it at startup.
2. **Meilisearch key.** A new `MEILI_MASTER_KEY` takes effect when Meilisearch restarts. If search is empty afterwards, run **Settings → Admin → Reindex search**.
3. **Volume ownership.** Files written by the old root container stay owned by root, and the new UID 10001 process cannot write them. Fix once:

   ```bash
   docker run --rm -v linkdean_avatars:/avatars -v linkdean_previews:/previews alpine \
     chown -R 10001:10001 /avatars /previews
   ```

4. **Migrations** run on API start. Migration `034` caps trash retention at 3,650 days and clamps larger values. Migration `035` removes decrypted vault titles from old activity entries. Migration `042` marks every existing account's email verified; see [Email](#email).
5. **Reverse proxy.** Set `TRUST_FORWARDED_FOR=true` if you run behind one.

## Development setup

The dev stack (`deploy/docker/docker-compose.dev.yml`) and `.env.example` use well-known secrets and set `ALLOW_DEFAULT_SECRETS=true`, so they start without generating anything. Never copy those values into production.

The dev and production Compose files share a project name and volumes. Run one at a time. A database volume first created by the dev stack keeps the password `linkdean`; follow [Upgrading an existing install](#upgrading-an-existing-install) step 1 before switching that volume to production.

Running the API and web on the host is covered in the README under Development.

## Healthchecks

| Probe | Path | Meaning |
| --- | --- | --- |
| Liveness | `GET /api/health` | Process up |
| Readiness | `GET /api/ready` | Postgres + Meilisearch reachable |
| Metrics | `GET /api/metrics` | Prometheus scrape. Off unless `METRICS_TOKEN` is set; then send `Authorization: Bearer <token>` |

`/api/ready` returns `true` or `false` per check. Failure details go to the API log, not the response.

Metrics are labelled by route template (such as `/api/l/{slug}`), never by raw path.

Compose marks `db`, `meilisearch`, and `api` healthy before dependents start. Example:

```bash
curl -fsS http://localhost:8080/api/health
curl -fsS http://localhost:8080/api/ready
```

## Backup

Stop writes briefly (or accept a short inconsistency window), then dump Postgres + Meili + volumes.

```bash
# from deploy/docker (compose project name: linkdean)
set -a; . ./.env; set +a
COMPOSE="docker compose -f docker-compose.yml"

# 1. Postgres
$COMPOSE exec -T db pg_dump -U linkdean -d linkdean -Fc > "linkdean-pg-$(date +%F).dump"

# 2. Meilisearch dump (creates a dump task; file lands under meili volume)
$COMPOSE exec -T meilisearch \
  wget -qO- --header="Authorization: Bearer ${MEILI_MASTER_KEY:?}" \
  --post-data='' http://127.0.0.1:7700/dumps
# After the dump task completes, copy from the meili volume, e.g.:
# docker run --rm -v linkdean_meili:/data -v "$PWD":/out alpine \
#   sh -c 'cp /data/dumps/*.dump /out/'

# 3. File volumes (avatars + previews)
docker run --rm \
  -v linkdean_avatars:/avatars \
  -v linkdean_previews:/previews \
  -v "$PWD":/out alpine \
  tar czf /out/linkdean-files-$(date +%F).tgz -C / avatars previews
```

Restore Postgres with `pg_restore` into a fresh DB, restore Meili dump per [Meilisearch dump docs](https://www.meilisearch.com/docs/learn/dump), then extract the files tarball back onto the named volumes and run the `chown` from [Upgrading](#upgrading-an-existing-install) step 3 if the tarball was made as root. After restore, use **Settings → Admin → Reindex search** if the Meili dump was skipped.
