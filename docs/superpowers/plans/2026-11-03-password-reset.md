# Password Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let people reset a forgotten password by email (30-minute single-use link), and let instance admins create a 24-hour reset link to hand over when mail is off.

**Architecture:** A `password_reset_tokens` table holds hashed tokens, one live per user. `crates/db` creates and consumes them in transactions (consuming sets the password, ends every session, optionally revokes API keys). The API adds `POST /api/auth/password/forgot`, `POST /api/auth/password/reset` and `POST /api/admin/users/{id}/reset-link`, and queues four new email templates through the part A outbox. The web app adds `/forgot-password`, `/reset-password` (token in the URL fragment) and an admin modal.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8), `linkdean-mail` (MJML templates), React + Mantine 9 + TanStack Router/Query + zod, vitest.

Spec: [`docs/superpowers/specs/2026-11-03-password-reset-design.md`](../specs/2026-11-03-password-reset-design.md). Builds on the mail foundation: `crate::mail::enqueue(state, to, template, &data) -> Result<Queued>` (`Queued::Queued | Queued::Disabled`), `state.mail: Option<Arc<Mail>>`, templates in `crates/mail/templates/` registered in `crates/mail/src/render.rs` `TEMPLATES`.

**Repo rules for every task:**

- Work on `main`. Commit each green task and push. Never commit red. Commit messages are normal prose ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Stage only the files you changed; never `git add -A`.
- Rust: always `cargo … -j 2`. Tests use only the throwaway Postgres: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test`. Never the dev database; never touch containers named `linkdean-*`.
- Rust format: `docker run --rm -v "$PWD":/w -w /w rust:1.89 sh -c "rustup component add rustfmt >/dev/null 2>&1; cargo fmt --all -- --check"`.
- Web, from `apps/web`: `npx vitest run`, `npx prettier --check src`, `npx oxlint src`, `npx tsc -b --noEmit`.
- UI copy, identifiers and docs in American spelling.
- After changing files, run `graphify update .` from the repo root. Do not commit `graphify-out/`.

---

## File map

| File | Responsibility |
| --- | --- |
| `migrations/041_password_reset_tokens.sql` (create) | Token table |
| `crates/db/src/repo/password_resets.rs` (create), `repo/mod.rs` (modify) | Create, consume, void, purge |
| `crates/db/src/repo/users.rs` (modify) | `change_password` voids live tokens; admin summary gains `has_password` |
| `crates/core/src/models/mod.rs` (modify) | `AdminUserSummary.has_password` |
| `crates/db/tests/password_resets.rs` (create) | Token tests |
| `crates/mail/templates/password_reset*.{mjml,subject,sample.json}`, `password_changed.*`, `password_reset_sso.*`, `password_reset_admin.*` (create); `crates/mail/src/render.rs` (modify) | Four emails |
| `crates/api/src/rate_limit.rs` (modify) | Reset limits |
| `crates/api/src/routes/auth/password_reset.rs` (create), `routes/auth/mod.rs` (modify) | Forgot and reset routes, `mail_enabled` in bootstrap status |
| `crates/api/src/routes/admin/mod.rs` (modify) | Admin reset link route |
| `crates/api/src/maintenance.rs` (modify) | Purge old tokens |
| `apps/web/src/schemas/auth/auth.ts`, `api/api.ts`, `hooks/auth/usePasswordReset.ts` (create), `hooks/auth/index.ts` | Client contract |
| `apps/web/src/pages/password-reset/{ForgotPasswordPage,ResetPasswordPage}.tsx` + tests (create), `pages/index.ts` | Pages |
| `apps/web/src/routes/forgot-password.tsx`, `routes/reset-password.tsx` (create) | Routes |
| `apps/web/src/pages/login/LoginPage.tsx` (modify) | Link or "ask your admin" |
| `apps/web/src/pages/settings/AdminResetLinkModal.tsx` + test (create), `AdminUsersPanel.tsx` (modify) | Admin flow |
| `README.md`, `docs/SELF_HOSTED.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md`, `docs/ROADMAP.md` | Docs |

---

### Task 1: Reset tokens in the database

**Files:** `migrations/041_password_reset_tokens.sql` (check `ls migrations | tail -1`; use the next free number), `crates/db/src/repo/password_resets.rs`, `crates/db/src/repo/mod.rs`, `crates/db/src/repo/users.rs`, `crates/db/tests/password_resets.rs`.

- [ ] **Step 1: Migration**

```sql
-- One live password reset link per user. Only the SHA-256 of the token is stored.
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    -- Set when an instance admin created the link instead of an email request.
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX password_reset_tokens_user ON password_reset_tokens (user_id) WHERE used_at IS NULL;
```

- [ ] **Step 2: Failing tests** — `crates/db/tests/password_resets.rs`. Use the `pool_or_skip()` pattern from `crates/db/tests/activity.rs` (shared `linkdean_test` DB is fine: every test makes its own user with a unique email). Create users with `db::create_user(&pool, &email, Some("old-password-1"), None)` — check its exact signature in `crates/db/src/repo/users.rs` and adapt.

```rust
//! Password reset tokens: single use, expiry, newest wins, and what a reset ends.

use linkdean_core::Error;
use linkdean_db::{self as db, ResetOutcome};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping password reset tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &sqlx::PgPool) -> Uuid {
    let email = format!("reset-{}@reset.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("old-password-1"), None).await.unwrap().id
}

#[tokio::test]
async fn a_token_resets_once_and_ends_every_session() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    db::create_session(&pool, uid, 30).await.unwrap();
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();

    let out = db::consume_reset_token(&pool, &token, "new-password-123", false).await.unwrap();
    assert_eq!(out, ResetOutcome { user_id: uid, signed_out_sessions: 1, revoked_api_keys: 0 });
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM sessions WHERE user_id = $1")
        .bind(uid).fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
    let email: (String,) = sqlx::query_as("SELECT email FROM users WHERE id = $1").bind(uid).fetch_one(&pool).await.unwrap();
    assert!(db::authenticate_local(&pool, &email.0, "new-password-123").await.is_ok());

    let again = db::consume_reset_token(&pool, &token, "another-password-1", false).await;
    assert!(matches!(again, Err(Error::BadRequest(_))), "{again:?}");
}

#[tokio::test]
async fn an_expired_token_is_refused() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();
    sqlx::query("UPDATE password_reset_tokens SET expires_at = now() - interval '1 second' WHERE user_id = $1")
        .bind(uid).execute(&pool).await.unwrap();
    assert!(matches!(db::consume_reset_token(&pool, &token, "new-password-123", false).await, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn a_new_token_voids_the_old_one() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    let old = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();
    let new = db::create_reset_token(&pool, uid, chrono::Duration::hours(24), Some(uid)).await.unwrap();
    assert!(db::consume_reset_token(&pool, &old, "new-password-123", false).await.is_err());
    assert!(db::consume_reset_token(&pool, &new, "new-password-123", false).await.is_ok());
}

#[tokio::test]
async fn changing_the_password_voids_a_live_token() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();
    db::change_password(&pool, uid, "old-password-1", "changed-password-1", None, false).await.unwrap();
    assert!(db::consume_reset_token(&pool, &token, "new-password-123", false).await.is_err());
}

#[tokio::test]
async fn two_concurrent_uses_only_one_wins() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();
    let (a, b) = tokio::join!(
        db::consume_reset_token(&pool, &token, "first-password-1", false),
        db::consume_reset_token(&pool, &token, "second-password-1", false),
    );
    assert_eq!([a.is_ok(), b.is_ok()].iter().filter(|ok| **ok).count(), 1);
}

#[tokio::test]
async fn a_short_password_is_refused_and_the_token_survives() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();
    assert!(matches!(db::consume_reset_token(&pool, &token, "short", false).await, Err(Error::BadRequest(_))));
    assert!(db::consume_reset_token(&pool, &token, "long-enough-password", false).await.is_ok());
}

#[tokio::test]
async fn old_tokens_are_purged() {
    let Some(pool) = pool_or_skip().await else { return };
    let uid = user(&pool).await;
    db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None).await.unwrap();
    sqlx::query("UPDATE password_reset_tokens SET expires_at = now() - interval '2 days' WHERE user_id = $1")
        .bind(uid).execute(&pool).await.unwrap();
    assert!(db::purge_old_reset_tokens(&pool).await.unwrap() >= 1);
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM password_reset_tokens WHERE user_id = $1")
        .bind(uid).fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}
```

Run: `DATABASE_URL=… cargo test -j 2 -p linkdean-db --test password_resets` → compile errors (functions missing).

- [ ] **Step 3: Implement** `crates/db/src/repo/password_resets.rs`:

```rust
//! Password reset links. The token is shown once; only its hash is stored.

use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_password, hash_token, MIN_SECRET_LEN};
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetOutcome {
    pub user_id: Uuid,
    pub signed_out_sessions: u64,
    pub revoked_api_keys: u64,
}

const INVALID: &str = "this reset link has expired or was already used";

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// A new link for `user_id`, valid for `ttl`. Any earlier unused link stops working.
pub async fn create_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    ttl: Duration,
    created_by: Option<Uuid>,
) -> Result<String> {
    let token = hex::encode(crypto::random_bytes(32));
    let mut tx = pool.begin().await.map_err(db_err)?;
    void_reset_tokens(&mut tx, user_id).await?;
    sqlx::query(
        "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at, created_by) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(hash_token(&token))
    .bind(Utc::now() + ttl)
    .bind(created_by)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(token)
}

/// Mark every unused link of `user_id` used, so none of them works any more.
pub(crate) async fn void_reset_tokens(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
) -> Result<()> {
    sqlx::query("UPDATE password_reset_tokens SET used_at = now() WHERE user_id = $1 AND used_at IS NULL")
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Set a new password with a reset link: ends every session of the user and, with
/// `revoke_api_keys`, revokes their API keys. The link works once.
pub async fn consume_reset_token(
    pool: &PgPool,
    token: &str,
    new_password: &str,
    revoke_api_keys: bool,
) -> Result<ResetOutcome> {
    if new_password.len() < MIN_SECRET_LEN {
        return Err(Error::BadRequest(format!("password min {MIN_SECRET_LEN} chars")));
    }
    let new_password = new_password.to_string();
    let new_hash = super::blocking::run_kdf(move || hash_password(&new_password)).await?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    // FOR UPDATE: a second use of the same link waits here, then finds it used.
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT t.id, t.user_id FROM password_reset_tokens t
        JOIN users u ON u.id = t.user_id
        WHERE t.token_hash = $1 AND t.used_at IS NULL AND t.expires_at > now()
          AND u.password_hash IS NOT NULL
        FOR UPDATE OF t
        "#,
    )
    .bind(hash_token(token))
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?;
    let Some((token_id, user_id)) = row else {
        return Err(Error::BadRequest(INVALID.into()));
    };
    sqlx::query("UPDATE password_reset_tokens SET used_at = now() WHERE id = $1")
        .bind(token_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .bind(new_hash)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let signed_out_sessions = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?
        .rows_affected();
    let revoked_api_keys = if revoke_api_keys {
        sqlx::query("UPDATE api_keys SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?
            .rows_affected()
    } else {
        0
    };
    tx.commit().await.map_err(db_err)?;
    Ok(ResetOutcome { user_id, signed_out_sessions, revoked_api_keys })
}

/// Links used or expired more than a day ago.
pub async fn purge_old_reset_tokens(pool: &PgPool) -> Result<u64> {
    Ok(sqlx::query(
        "DELETE FROM password_reset_tokens WHERE coalesce(used_at, expires_at) < now() - interval '1 day'",
    )
    .execute(pool)
    .await
    .map_err(db_err)?
    .rows_affected())
}
```

Notes:
- `hex` and `linkdean_core::crypto::random_bytes` are already used by `sessions.rs`; follow its imports. If `hash_password` / `MIN_SECRET_LEN` live elsewhere, import them as `users.rs` does.
- Register in `repo/mod.rs`: `mod password_resets;` and `pub use password_resets::*;` in alphabetical position.
- `users.rs` `change_password`: inside its transaction, before `tx.commit()`, call `super::password_resets::void_reset_tokens(&mut tx, user_id).await?;`.
- Admin summary: add `pub has_password: bool` to `AdminUserSummary` in `crates/core/src/models/mod.rs` (doc comment: "False for accounts that only sign in with SSO."), and `has_password: user.password_hash.is_some(),` in `admin_summary` (`users.rs`), set before `email` is moved.

- [ ] **Step 4:** Tests pass (7). Full `cargo test -j 2 -p linkdean-db` green. fmt check.
- [ ] **Step 5: Commit** "Store password reset links." Body: hashed single-use tokens, one live per user, consuming in one transaction sets the password and ends every session, password change voids live links, admin summaries say whether an account has a password.

---

### Task 2: Email templates

**Files:** `crates/mail/templates/` (create 4 × `.mjml`, `.subject`, `.sample.json`), `crates/mail/src/render.rs`.

Follow `crates/mail/templates/test.mjml`: extend `layout.mjml`, import `action` from `macros.mjml` inside `{% block content %}`, fill `title`, `preview`, `content`, `why`. Keep each to one heading, one or two short paragraphs, at most one button.

| Template | Variables | Subject | Content |
| --- | --- | --- | --- |
| `password_reset` | `reset_url`, `minutes` | `Reset your {{ site.instance }} password` | "Someone asked to reset the password for this account. The link works once, for {{ minutes }} minutes." Button "Reset your password" → `reset_url`. "If this wasn't you, ignore this email; your password stays the same." |
| `password_changed` | `changed_at` | `Your {{ site.instance }} password was changed` | "Your password was changed at {{ changed_at }} and every device was signed out." "Not you? Reset it again from the sign-in page and tell your admin." Button "Sign in" → `site.app_url ~ "/login"`. |
| `password_reset_sso` | — | `About your {{ site.instance }} sign-in` | "Someone asked to reset the password for this address, but this account has no password: it signs in with your organization's single sign-on." Button "Sign in" → `site.app_url ~ "/login"`. |
| `password_reset_admin` | — | `An admin created a password reset link for you` | "An admin of {{ site.instance }} created a password reset link for your account. They will send it to you. If you didn't ask for one, tell them." No button. |

Samples: `{"reset_url": "https://links.example.com/reset-password#token=abc123", "minutes": 30}`, `{"changed_at": "3 Nov 2026, 14:05 UTC"}`, `{}`, `{}`. `why` blocks: "You got this email because a password reset was requested for your account." (and the matching sentence for each).

Register all four in `TEMPLATES` (`template!("password_reset")`, …). Existing tests (`every_template_renders_its_sample`, text checks) cover them; add one test that `password_reset` HTML contains `href="https://links.example.com/reset-password#token=abc123"` and its text contains the URL. Check them visually at `/api/dev/emails` on the dev stack (it rebuilds on change).

- [ ] Commit "Add the password reset emails."

---

### Task 3: API routes

**Files:** `crates/api/src/rate_limit.rs`, `crates/api/src/routes/auth/password_reset.rs` (create), `crates/api/src/routes/auth/mod.rs`, `crates/api/src/routes/admin/mod.rs`, `crates/api/src/maintenance.rs`.

- [ ] **Step 1: Rate limits** in `rate_limit.rs` (same shape as `login_limits`):

```rust
const RESET_REQUEST_ACCOUNT: u32 = 3;
const RESET_REQUEST_ADDRESS: u32 = 10;
const RESET_ATTEMPT_ADDRESS: u32 = 10;

pub fn reset_request_limits(ip: IpAddr, email: &str) -> [Limit; 2] {
    let email = email.trim().to_ascii_lowercase();
    [
        (format!("reset-request-account:{email}"), RESET_REQUEST_ACCOUNT),
        (format!("reset-request-address:{ip}"), RESET_REQUEST_ADDRESS),
    ]
}

pub fn reset_attempt_limit(ip: IpAddr) -> [Limit; 1] {
    [(format!("reset-attempt:{ip}"), RESET_ATTEMPT_ADDRESS)]
}
```

Add unit tests next to the existing ones: keys differ per email and per IP; the email key is case-insensitive.

- [ ] **Step 2: Routes** — `crates/api/src/routes/auth/password_reset.rs`:

```rust
//! Forgotten passwords: request a link by email, then set a new password with it.

use super::session_response;
use crate::error::ApiResult;
use crate::mail::{enqueue, Queued};
use crate::rate_limit::{client_ip, reset_attempt_limit, reset_request_limits};
use crate::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use linkdean_core::{AuthResponse, Error};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;

/// How long an emailed link works.
pub const EMAIL_LINK_MINUTES: i64 = 30;

#[derive(Deserialize)]
pub struct ForgotRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetRequest {
    pub token: String,
    pub password: String,
    #[serde(default)]
    pub revoke_api_keys: bool,
}

pub fn reset_url(app_url: &str, token: &str) -> String {
    // The token rides in the fragment, which browsers never send to a server.
    format!("{}/reset-password#token={token}", app_url.trim_end_matches('/'))
}

/// Always answers the same, at once; the lookup and email happen in the background, so neither
/// the reply nor its timing says whether the address has an account.
pub async fn forgot(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ForgotRequest>,
) -> ApiResult<Json<Value>> {
    if state.mail.is_none() {
        return Err(Error::BadRequest("password reset by email is off; ask your admin for a reset link".into()).into());
    }
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = reset_request_limits(ip, &req.email);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let email = req.email.trim().to_ascii_lowercase();
    tokio::spawn(async move {
        if let Err(e) = send_reset_email(&state, &email).await {
            tracing::warn!(error = %e, "password reset email failed");
        }
    });
    Ok(Json(json!({ "ok": true, "minutes": EMAIL_LINK_MINUTES })))
}

async fn send_reset_email(state: &AppState, email: &str) -> linkdean_core::Result<()> {
    let Some(user) = db::find_user_by_email(&state.pool, email).await? else {
        return Ok(());
    };
    if user.password_hash.is_none() {
        enqueue(state, &user.email, "password_reset_sso", &json!({})).await?;
        return Ok(());
    }
    let token = db::create_reset_token(
        &state.pool,
        user.id,
        chrono::Duration::minutes(EMAIL_LINK_MINUTES),
        None,
    )
    .await?;
    let data = json!({ "reset_url": reset_url(&state.cfg.app_url, &token), "minutes": EMAIL_LINK_MINUTES });
    if enqueue(state, &user.email, "password_reset", &data).await? == Queued::Disabled {
        tracing::warn!("password reset requested while mail is off");
    }
    Ok(())
}

/// Set the new password, sign this browser in and end every other session.
pub async fn reset(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = reset_attempt_limit(ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let out = db::consume_reset_token(&state.pool, req.token.trim(), &req.password, req.revoke_api_keys).await?;
    state.clear_vault_keys(out.user_id).await;
    let user = db::find_user_by_id(&state.pool, out.user_id).await?.ok_or(Error::NotFound)?;
    let changed_at = chrono::Utc::now().format("%-d %b %Y, %H:%M UTC").to_string();
    if let Err(e) = enqueue(&state, &user.email, "password_changed", &json!({ "changed_at": changed_at })).await {
        tracing::warn!(error = %e, "password changed email failed");
    }
    let token = db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(session_response(
        AuthResponse { user: public, token: token.clone() },
        &token,
        state.cfg.session_days,
        state.cfg.cookie_secure(),
    ))
}
```

Notes:
- `mail::enqueue` and `Queued` carry `#[allow(dead_code)]` from part A; remove those attributes now that they are used.
- In `routes/auth/mod.rs`: `mod password_reset;`, make `session_response` `pub(super)` (or `pub(crate)`), add routes `.route("/auth/password/forgot", post(password_reset::forgot))` and `.route("/auth/password/reset", post(password_reset::reset))`, and add `"mail_enabled": state.mail.is_some()` to both `bootstrap_status` and `auth_config` JSON.
- A banned user may reset; the session created here is refused by the `AuthUser` extractor's ban check, so the web app lands on the banned page. Nothing extra needed.
- These POSTs come from the signed-out web app, so they carry `X-Linkdean-Client: web` like every other request from `api/client.ts`; no auth extractor is involved.

- [ ] **Step 3: Admin link** — in `routes/admin/mod.rs` add `.route("/admin/users/{id}/reset-link", post(create_reset_link))`:

```rust
/// How long an admin-made link works: long enough to hand it over.
const ADMIN_LINK_HOURS: i64 = 24;

async fn create_reset_link(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let user = db::find_user_by_id(&state.pool, id).await?.ok_or(Error::NotFound)?;
    if user.password_hash.is_none() {
        return Err(Error::BadRequest("this account signs in with SSO and has no password".into()).into());
    }
    let token = db::create_reset_token(&state.pool, id, chrono::Duration::hours(ADMIN_LINK_HOURS), Some(admin.id)).await?;
    if let Err(e) = crate::mail::enqueue(&state, &user.email, "password_reset_admin", &json!({})).await {
        tracing::warn!(error = %e, "admin reset notice email failed");
    }
    let url = crate::routes::auth::reset_url(&state.cfg.app_url, &token);
    Ok(Json(json!({ "url": url, "expires_in_hours": ADMIN_LINK_HOURS })))
}
```

Expose `reset_url` as `pub use password_reset::reset_url;` from `routes/auth/mod.rs` and make `routes::auth` reachable as `crate::routes::auth` (it is a private `mod auth;` in `routes/mod.rs`; change to `pub(crate) mod auth;`). Import `Error` from `linkdean_core` in admin if not already.

- [ ] **Step 4: Maintenance** — in `maintenance.rs` `run_once`, after the outbox purge:

```rust
    match linkdean_db::purge_old_reset_tokens(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(tokens = n, "purged old password reset links"),
        Err(e) => tracing::warn!(error = %e, "password reset link purge failed"),
    }
```

- [ ] **Step 5: Tests and smoke.** Unit-test `reset_url` (fragment, no double slash). Build, then run the API against the throwaway `smoke` DB with a throwaway Mailpit (`docker run -d --rm --name ld-mailpit -p 127.0.0.1:11025:1025 -p 127.0.0.1:18025:8025 axllent/mailpit:v1.31.0`; env as in `/tmp/claude-502/ldsmoke/run.sh on on`). With curl (every POST sends `X-Linkdean-Client: web`):
  - `POST /api/auth/password/forgot` for a known and an unknown address → identical 200 bodies; Mailpit receives exactly one `password_reset` mail, for the known address.
  - Take the token from that email's text, `POST /api/auth/password/reset` → 200 with a `Set-Cookie`; the old password fails at `/api/auth/login`, the new one works; Mailpit has a `password_changed` mail; reusing the token → 400.
  - 4th forgot request for the same address within 15 minutes → 429.
  - Admin (temporarily `UPDATE users SET is_admin = true` for `uiqa-theme@smoke.test` in the `smoke` DB, revert afterwards): `POST /api/admin/users/{id}/reset-link` → `url` with `#token=`; a non-admin → 403.
  - Mail off (restart without SMTP vars): forgot → 400; `GET /api/auth/bootstrap-status` → `mail_enabled: false`; the admin link still works.
  - Create test users with a `uiqa-reset-` email prefix and delete them afterwards. Stop the API and Mailpit.
- [ ] Full `cargo test -j 2 --workspace` green, fmt check. **Commit** "Reset forgotten passwords by email or with an admin link."

---

### Task 4: Web pages

**Files:** `apps/web/src/schemas/auth/auth.ts`, `api/api.ts`, `hooks/auth/usePasswordReset.ts`, `hooks/auth/index.ts`, `pages/password-reset/*`, `pages/index.ts`, `routes/forgot-password.tsx`, `routes/reset-password.tsx`, `pages/login/LoginPage.tsx`, `pages/settings/AdminResetLinkModal.tsx`, `pages/settings/AdminUsersPanel.tsx`.

- [ ] **Contract:**
  - `bootstrapStatusSchema`: add `mail_enabled: z.boolean().default(false)`.
  - `adminUserSummarySchema`: add `has_password: z.boolean().default(true)`.
  - New schemas: `forgotPasswordResponseSchema = z.object({ ok: z.boolean(), minutes: z.number() })`, `adminResetLinkSchema = z.object({ url: z.string(), expires_in_hours: z.number() })`.
  - `api.ts`: `forgotPassword(email)` → POST `/auth/password/forgot`; `resetPassword({ token, password, revoke_api_keys })` → POST `/auth/password/reset` with `authTokenResponseSchema` (as `login` does); `createResetLink(userId)` → POST `/admin/users/${id}/reset-link`.
  - Hooks in `hooks/auth/usePasswordReset.ts`: `useForgotPassword()`, `useResetPassword()` (on success: `markSignedIn(true)`, invalidate `['me']`, as `useAuth`'s login does), `useCreateResetLink()`. Export them.
- [ ] **Login page:** under the password field, when `bootstrap.data?.mail_enabled`, an `Anchor` "Forgot password?" navigating to `/forgot-password`; otherwise dimmed text "Forgot your password? Ask your admin for a reset link."
- [ ] **ForgotPasswordPage** (`/forgot-password`, same centered layout as `LoginPage`: `Stack maw={420} mx="auto" mt={80}`, LogoMark + title): email field, "Send reset link" button. After success, replace the form with an `Alert`: "If an account exists for that address, we've sent a link. It works for 30 minutes." (use `minutes` from the reply) and a "Back to sign in" link. 429 shows the API message.
- [ ] **ResetPasswordPage** (`/reset-password`): read the token from `window.location.hash` (`new URLSearchParams(hash.slice(1)).get('token')`) once on mount, then `history.replaceState` to drop the fragment so the token leaves the address bar. Fields: "New password", "Repeat new password" (must match, min 12 chars: reuse the constant the register page uses), Checkbox "Also revoke my API keys". On success navigate to `/library`. On a 400 whose message contains "expired or was already used", or with no token, show "This link has expired or was already used." with a button "Request a new link" → `/forgot-password`.
- [ ] **Routes:** `routes/forgot-password.tsx`, `routes/reset-password.tsx` with `createFileRoute`, like `routes/login.tsx`. Signed-in users visiting them are fine (no redirect).
- [ ] **Admin:** in `AdminUsersPanel.tsx`, for each user with `has_password` (including yourself), a `Button size="xs" variant="subtle"` "Reset link" that opens `AdminResetLinkModal`. The modal calls `useCreateResetLink` when the admin clicks "Create link" (not on open), then shows the URL in a read-only `TextInput` with Mantine `CopyButton`, the note "Send it privately. Anyone with this link can set the password. It works once, for 24 hours. Creating a new link cancels the previous one." and, when mail is on, "We also emailed them that a link was created."
- [ ] **Tests** (vitest + RTL, mock `@/hooks` as `AdminEmailPanel.test.tsx` does):
  - Login shows the link with mail on, the "ask your admin" text with mail off.
  - Forgot page shows the same confirmation after submit.
  - Reset page: mismatch blocks submit; no token shows the expired state; a successful submit calls the mutation with the token and the checkbox value.
  - Admin modal: creates only on click and shows the URL with a copy button.
- [ ] All web checks green. **Commit** "Add the forgot and reset password pages and the admin reset link."

---

### Task 5: Docs, browser check, close-out

- [ ] **Docs** (follow the `linkdean-self-host-docs` skill; derive values from code):
  - SELF_HOSTED `#email`: what password reset needs; the admin link as the fallback with mail off.
  - ARCHITECTURE Auth & sessions: reset links (hashed, single use, 30 min by email, 24 h by admin, newest wins, a reset ends every session and locks vaults), the fragment token, rate limits (`rate_limit.rs`), the background lookup that keeps replies identical.
  - FEATURES: move the password reset bullet from ROADMAP item 1 into FEATURES (Auth section).
  - README: mention reset in the feature list if the auth features are listed there.
  - Also update `docs/ARCHITECTURE.md`'s rate limit bullet list.
- [ ] **Browser check** (skill `browser-ui-verification`), on the throwaway stack (API on `smoke` with Mailpit, Vite on :5199, Playwright at `/tmp/claude-502/ldsmoke/pw`):
  1. Login → "Forgot password?" → submit a `uiqa-reset-` test user's address → confirmation.
  2. Fetch the email from Mailpit, open its link → set a new password → lands on `/library` signed in; the URL no longer shows the token.
  3. Open the same link again → expired state.
  4. Mail off: login shows "Ask your admin"; as admin, Settings → Admin → user → Reset link → Create link → copy → open it in a fresh context → set a password → signed in.
  5. No unexplained 4xx, no console errors; screenshot the reset page and look at it.
  Clean up test users and revert admin flags; stop servers and Mailpit.
- [ ] **Commit** "Document password reset." Run `graphify update .`.
