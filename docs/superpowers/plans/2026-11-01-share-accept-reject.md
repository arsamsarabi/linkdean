# Share Accept and Reject Implementation Plan

> **For agentic workers:** Use executing-plans. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sharing sends an invite, and access begins only when the invitee accepts.

**Architecture:** `pending_share_invites` becomes the single invite table for known and unknown invitees. The move into `category_shares` or `dashboard_shares` happens only on accept, through the existing `apply_category_claim` and `apply_dashboard_claim`. No read path changes.

**Tech Stack:** Rust, axum, sqlx, Postgres 16; React, Mantine, TanStack Query, zod, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-01-share-accept-reject-design.md`

---

## Conventions

- Work on `main`. Commit each green task and push. Never commit red.
- `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.
- DB tests skip when `DATABASE_URL` is unset. Use a throwaway container, never the dev database:

```bash
docker rm -f linkdean-pg-check >/dev/null 2>&1
docker run -d --name linkdean-pg-check -e POSTGRES_USER=linkdean -e POSTGRES_PASSWORD=linkdean -e POSTGRES_DB=linkdean -p 127.0.0.1:55433:5432 --tmpfs /var/lib/postgresql/data postgres:16-alpine
export DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:55433/linkdean
```

- Run `graphify update .` after each task.
- Commit messages are one sentence ending in a period, plus `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

### Task 1: Migration and models

**Files:** Create `migrations/027_share_invite_acceptance.sql`. Modify `crates/core/src/models/mod.rs`.

- [ ] **Step 1: Add the migration**

```sql
-- Invites now cover existing accounts too. Access starts when the invitee accepts.

ALTER TABLE pending_share_invites
    ADD COLUMN invitee_user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    ADD COLUMN rejected_at TIMESTAMPTZ;

UPDATE pending_share_invites p
SET invitee_user_id = u.id
FROM users u
WHERE lower(u.email) = p.invitee_email
  AND p.claimed_at IS NULL
  AND p.revoked_at IS NULL;

CREATE INDEX pending_share_invites_invitee_open_idx
    ON pending_share_invites (invitee_user_id)
    WHERE claimed_at IS NULL AND revoked_at IS NULL;
```

- [ ] **Step 2: Add the models**

Add to `struct PendingShareInvite`, after `revoked_at`:

```rust
    pub invitee_user_id: Option<Uuid>,
    pub rejected_at: Option<DateTime<Utc>>,
```

Add after `struct PendingShareInviteView`:

```rust
/// An invite as its invitee sees it. `rejected_at` is set when they declined it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInviteInboxItem {
    pub id: Uuid,
    pub subject_type: PendingShareSubjectType,
    pub subject_id: Uuid,
    pub subject_title: String,
    pub access: ShareAccess,
    pub inviter_display_name: String,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<DateTime<Utc>>,
}
```

- [ ] **Step 3: Verify and commit**

Run: `cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED"`
Expected: no failures.

Commit: `Add invitee and rejection columns to share invites.`

---

### Task 2: Invite lifecycle

**Files:** Modify `crates/db/src/repo/pending_shares.rs`, `crates/db/src/repo/blocks.rs`, `crates/api/src/routes/auth/mod.rs:94,305`. Test: create `crates/db/tests/share_invites.rs`.

- [ ] **Step 1: Write the failing test**

Create `crates/db/tests/share_invites.rs` with the usual `pool_or_skip`, `user(pool, prefix)` and `category(pool, owner)` helpers from `crates/db/tests/profile_visibility.rs`, plus `invite(pool, owner, cat, email)` calling `db::create_share` with `ShareAccess::Read`. Cover:

```rust
#[tokio::test]
async fn invite_grants_nothing_until_accepted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;

    let outcome = invite(&pool, &owner, cat, &invitee.email).await.expect("invite");
    assert!(matches!(outcome, CreateCategoryShareOutcome::Pending { .. }));
    assert!(db::get_category(&pool, invitee.id, cat).await.is_err());

    let inbox = db::list_share_invites(&pool, invitee.id).await.expect("inbox");
    assert_eq!(inbox.len(), 1);
    db::accept_share_invite(&pool, invitee.id, inbox[0].id).await.expect("accept");

    assert!(db::get_category(&pool, invitee.id, cat).await.is_ok());
    assert!(db::list_share_invites(&pool, invitee.id).await.expect("inbox").is_empty());
    let again = db::accept_share_invite(&pool, invitee.id, inbox[0].id).await;
    assert!(matches!(again, Err(Error::NotFound)));
}

#[tokio::test]
async fn rejecting_is_silent_blocks_reinvite_and_can_be_restored() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email).await.expect("invite");
    let id = db::list_share_invites(&pool, invitee.id).await.expect("inbox")[0].id;

    db::reject_share_invite(&pool, invitee.id, id).await.expect("reject");

    let pending = db::list_pending_for_subject(&pool, owner.id, PendingShareSubjectType::Category, cat)
        .await
        .expect("owner list");
    assert!(pending.is_empty());
    assert!(db::list_notifications(&pool, owner.id, 20).await.expect("owner notes").is_empty());

    let denied = invite(&pool, &owner, cat, &invitee.email).await;
    assert!(matches!(denied, Err(Error::Conflict(_))));

    let inbox = db::list_share_invites(&pool, invitee.id).await.expect("inbox");
    assert!(inbox[0].rejected_at.is_some());
    assert!(matches!(
        db::accept_share_invite(&pool, invitee.id, id).await,
        Err(Error::NotFound)
    ));

    db::restore_share_invite(&pool, invitee.id, id).await.expect("restore");
    assert!(db::get_category(&pool, invitee.id, cat).await.is_err());
    db::accept_share_invite(&pool, invitee.id, id).await.expect("accept");
    assert!(db::get_category(&pool, invitee.id, cat).await.is_ok());
}

#[tokio::test]
async fn other_users_cannot_act_on_an_invite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let stranger = user(&pool, "stranger").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email).await.expect("invite");
    let id = db::list_share_invites(&pool, invitee.id).await.expect("inbox")[0].id;

    for result in [
        db::accept_share_invite(&pool, stranger.id, id).await,
        db::reject_share_invite(&pool, stranger.id, id).await,
        db::accept_share_invite(&pool, owner.id, id).await,
    ] {
        assert!(matches!(result, Err(Error::NotFound)));
    }
}

#[tokio::test]
async fn signup_links_the_invite_without_granting() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("later-{}@invites.test", Uuid::new_v4());
    invite(&pool, &owner, cat, &email).await.expect("invite");

    let later = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("register");
    let linked = db::link_pending_share_invites(&pool, &email, later.id).await.expect("link");
    assert_eq!(linked, 1);

    assert!(db::get_category(&pool, later.id, cat).await.is_err());
    assert_eq!(db::list_share_invites(&pool, later.id).await.expect("inbox").len(), 1);
}

#[tokio::test]
async fn changing_email_keeps_the_invite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email).await.expect("invite");
    sqlx::query("UPDATE users SET email = $2 WHERE id = $1")
        .bind(invitee.id)
        .bind(format!("moved-{}@invites.test", Uuid::new_v4()))
        .execute(&pool)
        .await
        .expect("change email");

    let id = db::list_share_invites(&pool, invitee.id).await.expect("inbox")[0].id;
    db::accept_share_invite(&pool, invitee.id, id).await.expect("accept");
    assert!(db::get_category(&pool, invitee.id, cat).await.is_ok());
}

#[tokio::test]
async fn blocking_revokes_open_invites_including_rejected() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email).await.expect("invite");
    let id = db::list_share_invites(&pool, invitee.id).await.expect("inbox")[0].id;
    db::reject_share_invite(&pool, invitee.id, id).await.expect("reject");

    db::block_user(&pool, invitee.id, owner.id).await.expect("block");
    assert!(db::list_share_invites(&pool, invitee.id).await.expect("inbox").is_empty());
}

#[tokio::test]
async fn accepting_after_the_subject_is_gone_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email).await.expect("invite");
    let id = db::list_share_invites(&pool, invitee.id).await.expect("inbox")[0].id;

    db::delete_category(&pool, owner.id, cat).await.expect("delete");
    assert!(db::list_share_invites(&pool, invitee.id).await.expect("inbox").is_empty());
    assert!(matches!(
        db::accept_share_invite(&pool, invitee.id, id).await,
        Err(Error::NotFound)
    ));
}
```

Add a dashboard case too, using `db::create_dashboard_share`, that asserts `db::get_dashboard` fails before accept and works after.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p linkdean-db --test share_invites`
Expected: compile error, `list_share_invites` not found.

- [ ] **Step 3: Implement in `pending_shares.rs`**

`upsert_pending_share_invite` gains `invitee_user_id: Option<Uuid>` and returns `Result<(PendingShareInviteView, bool)>`, where the bool is true for a new invite. Before the insert:

```rust
    if let Some(existing) = find_open_invite(pool, subject_type, subject_id, &email, invitee_user_id).await? {
        if existing.rejected_at.is_some() {
            return Err(Error::Conflict("This invite can't be sent.".into()));
        }
        let row = sqlx::query_as::<_, PendingShareInvite>(
            "UPDATE pending_share_invites SET access = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(access.as_str())
        .bind(existing.id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        return Ok((to_view(row)?, false));
    }
```

Insert `invitee_user_id` as a new column in the `INSERT`, and return `(to_view(row)?, true)`.

Add:

```rust
async fn find_open_invite(
    pool: &PgPool,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
    email: &str,
    invitee_user_id: Option<Uuid>,
) -> Result<Option<PendingShareInvite>> {
    sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE subject_type = $1
          AND subject_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
          AND (invitee_email = $3 OR invitee_user_id = $4)
        ORDER BY rejected_at NULLS LAST
        LIMIT 1
        "#,
    )
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(email)
    .bind(invitee_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}
```

Add `AND rejected_at IS NULL` to `list_pending_for_subject`, `update_pending_share_invite` and `revoke_pending_share_invite`, so the owner cannot see or touch a rejected invite.

Delete both `notify_*` blocks from `apply_category_claim` and `apply_dashboard_claim`.

Replace `claim_pending_share_invites` with:

```rust
/// After register / OIDC signup: attach invites sent to this email. Access still needs acceptance.
pub async fn link_pending_share_invites(pool: &PgPool, email: &str, user_id: Uuid) -> Result<u32> {
    let email = normalize_email(email)?;
    let rows = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE invitee_email = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at ASC
        "#,
    )
    .bind(&email)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut linked = 0u32;
    for row in rows {
        if super::blocks::is_blocked_either_way(pool, user_id, row.invited_by_user_id).await? {
            revoke_invite(pool, row.id).await?;
            continue;
        }
        sqlx::query("UPDATE pending_share_invites SET invitee_user_id = $2, updated_at = now() WHERE id = $1")
            .bind(row.id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        notify_invitee(pool, &row, user_id).await?;
        linked += 1;
    }
    Ok(linked)
}

async fn revoke_invite(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET revoked_at = now(), updated_at = now()
        WHERE id = $1 AND claimed_at IS NULL AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Title of the shared subject, or `None` when it can no longer be shared.
async fn invite_subject_title(pool: &PgPool, row: &PendingShareInvite) -> Result<Option<String>> {
    match PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)? {
        PendingShareSubjectType::Category => {
            match require_category_owner(pool, row.invited_by_user_id, row.subject_id).await {
                Ok(c) if !c.is_vault && c.deleted_at.is_none() => Ok(Some(c.name)),
                Ok(_) | Err(Error::NotFound | Error::Forbidden) => Ok(None),
                Err(e) => Err(e),
            }
        }
        PendingShareSubjectType::Dashboard => {
            match require_dashboard_owner(pool, row.invited_by_user_id, row.subject_id).await {
                Ok(d) if d.deleted_at.is_none() => Ok(Some(d.title)),
                Ok(_) | Err(Error::NotFound | Error::Forbidden) => Ok(None),
                Err(e) => Err(e),
            }
        }
    }
}

async fn notify_invitee(pool: &PgPool, row: &PendingShareInvite, invitee_user_id: Uuid) -> Result<()> {
    let Some(title) = invite_subject_title(pool, row).await? else {
        return Ok(());
    };
    let access = ShareAccess::parse(&row.access).map_err(Error::BadRequest)?;
    match PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)? {
        PendingShareSubjectType::Category => {
            super::notifications::notify_category_share_invite(
                pool, row.invited_by_user_id, invitee_user_id, row.subject_id, &title, access,
            )
            .await;
        }
        PendingShareSubjectType::Dashboard => {
            super::notifications::notify_dashboard_share_invite(
                pool, row.invited_by_user_id, invitee_user_id, row.subject_id, &title, access,
            )
            .await;
        }
    }
    Ok(())
}

async fn find_invitee_row(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<PendingShareInvite> {
    sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE id = $1
          AND invitee_user_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

pub async fn list_share_invites(pool: &PgPool, user_id: Uuid) -> Result<Vec<ShareInviteInboxItem>> {
    let rows = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE invitee_user_id = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(subject_title) = invite_subject_title(pool, &row).await? else {
            continue;
        };
        let inviter_display_name = match find_user_by_id(pool, row.invited_by_user_id).await? {
            Some(u) => user_share_label(&u),
            None => "User".into(),
        };
        out.push(ShareInviteInboxItem {
            id: row.id,
            subject_type: PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)?,
            subject_id: row.subject_id,
            subject_title,
            access: ShareAccess::parse(&row.access).map_err(Error::BadRequest)?,
            inviter_display_name,
            created_at: row.created_at,
            rejected_at: row.rejected_at,
        });
    }
    Ok(out)
}

pub async fn accept_share_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = find_invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_some() {
        return Err(Error::NotFound);
    }
    if super::blocks::is_blocked_either_way(pool, user_id, row.invited_by_user_id).await? {
        revoke_invite(pool, row.id).await?;
        return Err(Error::NotFound);
    }

    // Claim first: a double click or an owner revoke then loses the race cleanly.
    let claimed = sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET claimed_at = now(), claimed_by_user_id = $2, updated_at = now()
        WHERE id = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
          AND rejected_at IS NULL
        "#,
    )
    .bind(row.id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if claimed.rows_affected() == 0 {
        return Err(Error::NotFound);
    }

    let applied = match PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)? {
        PendingShareSubjectType::Category => apply_category_claim(pool, &row, user_id, &row.access).await,
        PendingShareSubjectType::Dashboard => apply_dashboard_claim(pool, &row, user_id, &row.access).await,
    };
    match applied {
        Ok(true) => Ok(()),
        Ok(false) => {
            release_claim(pool, row.id, true).await?;
            Err(Error::NotFound)
        }
        Err(e) => {
            release_claim(pool, row.id, false).await?;
            Err(e)
        }
    }
}

async fn release_claim(pool: &PgPool, id: Uuid, revoke: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET claimed_at = NULL,
            claimed_by_user_id = NULL,
            revoked_at = CASE WHEN $2 THEN now() ELSE revoked_at END,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(revoke)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn reject_share_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = find_invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_some() {
        return Err(Error::NotFound);
    }
    set_rejected(pool, row.id, true).await
}

pub async fn restore_share_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = find_invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_none() {
        return Err(Error::NotFound);
    }
    set_rejected(pool, row.id, false).await
}

async fn set_rejected(pool: &PgPool, id: Uuid, rejected: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET rejected_at = CASE WHEN $2 THEN now() END, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(rejected)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}
```

Add `ShareInviteInboxItem` to the `linkdean_core` import and `user_share_label` to the users import.

In `crates/api/src/routes/auth/mod.rs:94,305`, rename the call to `link_pending_share_invites`, and rename `claimed` to `linked` in the log lines.

- [ ] **Step 4: Update the block revoke**

In `crates/db/src/repo/blocks.rs`, change the pending-invite `WHERE` to:

```sql
        WHERE claimed_at IS NULL
          AND revoked_at IS NULL
          AND (
            (invited_by_user_id = $1 AND (invitee_email = $2 OR invitee_user_id = $5))
            OR (invited_by_user_id = $3 AND (invitee_email = $4 OR invitee_user_id = $6))
          )
```

and add `.bind(blocked_id).bind(blocker_id)` after the existing binds. Change the comment to say "by email or account".

- [ ] **Step 5: Run the tests**

Run: `cargo test -p linkdean-db --test share_invites`
Expected: compile errors until Task 3 changes the `upsert` callers. Do Task 3's Step 1 before running, and commit Tasks 2 and 3 together.

---

### Task 3: Sharing creates an invite

**Files:** Modify `crates/db/src/repo/shares.rs`, `dashboard_shares.rs`, `notifications.rs`, and the DB tests below.

- [ ] **Step 1: Change `create_share` and `create_dashboard_share`**

For the unknown-email branch, pass `None` as `invitee_user_id` and take `.0` of the result. After `assert_not_blocked` and the `existed` check, add to `create_share`:

```rust
    if !existed {
        let (invite, created) = super::pending_shares::upsert_pending_share_invite(
            pool,
            owner_user_id,
            PendingShareSubjectType::Category,
            category_id,
            &email,
            req.access,
            Some(invitee.id),
        )
        .await?;
        if created {
            super::notifications::notify_category_share_invite(
                pool, owner_user_id, invitee.id, category_id, &cat.name, invite.access,
            )
            .await;
        }
        return Ok(CreateCategoryShareOutcome::Pending { invite });
    }
```

Remove the `if existed` / `else` around the later notification, leaving only `notify_category_share_access`. Make the same change in `create_dashboard_share` with `PendingShareSubjectType::Dashboard`, `dashboard_id`, `dash.title`, `notify_dashboard_share_invite`, and its access notification.

- [ ] **Step 2: Change the notification wording**

In `notifications.rs:157` and `:233`, change the bodies to `{actor_label} invited you to folder “{category_name}” ({access_label})` and `{actor_label} invited you to dashboard “{dashboard_title}” ({access_label})`.

- [ ] **Step 3: Move existing tests onto accept**

Add this helper to each test file below:

```rust
async fn accept_invites(pool: &sqlx::PgPool, invitee: Uuid) {
    for invite in db::list_share_invites(pool, invitee).await.expect("inbox") {
        db::accept_share_invite(pool, invitee, invite.id).await.expect("accept");
    }
}
```

| File | Change |
| --- | --- |
| `crates/db/tests/authz.rs:254,475` | Call `accept_invites` after `create_share`, drop `expect_active_share`, and read the share back through the existing owner list |
| `crates/db/tests/authz.rs:622`, `blocks.rs:294,334` | Use `link_pending_share_invites`, then assert no access, then `accept_invites`, then assert access |
| `crates/db/tests/blocks.rs:57` | `share` helper takes the invitee id and calls `accept_invites`, except where a test only checks the blocked error |
| `crates/db/tests/blocks.rs:147`, `dashboards.rs:354`, `notifications.rs:171,263`, `profile_visibility.rs:519` | Call `accept_invites` after sharing. In `dashboards.rs`, replace `expect_active_dash_share` the same way as authz |
| `crates/db/tests/notifications.rs:66` | No change: it asserts the invite notification, which is still sent at invite time |

- [ ] **Step 4: Run everything**

Run: `cargo fmt --all && cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: no failures, including every `share_invites` test.

- [ ] **Step 5: Commit**

Commit: `Make sharing send an invite that the invitee accepts or rejects.`

---

### Task 4: Routes

**Files:** Create `crates/api/src/routes/share_invites/mod.rs`. Modify `crates/api/src/routes/mod.rs`.

- [ ] **Step 1: Add the routes**

```rust
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/share-invites", get(list))
        .route("/share-invites/{id}/accept", post(accept))
        .route("/share-invites/{id}/reject", post(reject))
        .route("/share-invites/{id}/restore", post(restore))
}

async fn list(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    let invites = db::list_share_invites(&state.pool, user.id).await?;
    Ok(Json(json!({ "invites": invites })))
}

async fn accept(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::accept_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn reject(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::reject_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn restore(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::restore_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
```

In `routes/mod.rs`, declare the module alongside the others and add `.merge(share_invites::router())` after `.merge(notifications::router())`.

- [ ] **Step 2: Verify and commit**

Run: `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -cE "^crates.*(warning|error)"`
Expected: `0`.

Commit: `Add the share invite routes.`

---

### Task 5: Web data layer

**Files:** Create `apps/web/src/schemas/share-invites/` and `apps/web/src/hooks/share-invites/`. Modify `schemas/index.ts`, `hooks/index.ts`, `api/api.ts`.

- [ ] **Step 1: Add the schema**

`schemas/share-invites/shareInvites.ts`:

```ts
import { z } from 'zod'

export const shareInviteSchema = z.object({
  id: z.string(),
  subject_type: z.enum(['category', 'dashboard']),
  subject_id: z.string(),
  subject_title: z.string(),
  access: z.enum(['read', 'write']),
  inviter_display_name: z.string(),
  created_at: z.string(),
  rejected_at: z.string().nullish(),
})

export const shareInvitesEnvelopeSchema = z.object({
  invites: z.array(shareInviteSchema),
})

export type ShareInvite = z.infer<typeof shareInviteSchema>
```

Add `index.ts` and export it from `schemas/index.ts`.

- [ ] **Step 2: Add the API calls**

In `api/api.ts`, using the file's existing `okSchema` equivalent for `{ ok: boolean }` responses (`markAllReadSchema` accepts that shape if no plain one exists):

```ts
  shareInvites: (): Promise<{ invites: ShareInvite[] }> => req('/share-invites', {}, shareInvitesEnvelopeSchema),

  acceptShareInvite: (id: string): Promise<unknown> => req(`/share-invites/${id}/accept`, { method: 'POST' }),

  rejectShareInvite: (id: string): Promise<unknown> => req(`/share-invites/${id}/reject`, { method: 'POST' }),

  restoreShareInvite: (id: string): Promise<unknown> => req(`/share-invites/${id}/restore`, { method: 'POST' }),
```

- [ ] **Step 3: Add the hooks**

`hooks/share-invites/useShareInvites.ts`:

```ts
import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { ShareInvite } from '@/schemas'

export function useShareInvites(enabled = true): UseQueryResult<{ invites: ShareInvite[] }> {
  return useQuery({
    queryKey: ['share-invites'],
    queryFn: () => api.shareInvites(),
    enabled,
    refetchInterval: 30_000,
  })
}

function useInviteAction(action: (id: string) => Promise<unknown>): UseMutationResult<unknown, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: action,
    onSuccess: () => {
      for (const key of ['share-invites', 'categories', 'bookmarks', 'dashboards', 'notifications']) {
        void qc.invalidateQueries({ queryKey: [key] })
      }
    },
  })
}

export function useAcceptShareInvite(): UseMutationResult<unknown, Error, string> {
  return useInviteAction(api.acceptShareInvite)
}

export function useRejectShareInvite(): UseMutationResult<unknown, Error, string> {
  return useInviteAction(api.rejectShareInvite)
}

export function useRestoreShareInvite(): UseMutationResult<unknown, Error, string> {
  return useInviteAction(api.restoreShareInvite)
}
```

Add `index.ts` and export it from `hooks/index.ts`.

- [ ] **Step 4: Verify and commit**

Run: `cd apps/web && pnpm exec tsc -b 2>&1 | grep -c "error TS"`
Expected: `0`.

Commit: `Add the web data layer for share invites.`

---

### Task 6: Invite UI

**Files:** Create `components/notifications/ShareInvitesList.tsx` and its test, `pages/profile/ProfileDeclinedInvitesCard.tsx`. Modify `components/notifications/NotificationsDrawer.tsx`, `pages/app-shell/AppShell.tsx`, `pages/profile/ProfilePage.tsx`, `hooks/categories/useShareCategoryDrawer.ts:57`, `components/dashboard/ShareDashboardDrawer.tsx:43`.

- [ ] **Step 1: Write the failing test**

`components/notifications/ShareInvitesList.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ShareInvite } from '@/schemas'
import { ShareInvitesList } from './ShareInvitesList'

const accept = vi.hoisted(() => vi.fn())
const reject = vi.hoisted(() => vi.fn())
const invites = vi.hoisted(() => ({ list: [] as ShareInvite[] }))

vi.mock('@/hooks', () => ({
  useShareInvites: () => ({ data: { invites: invites.list } }),
  useAcceptShareInvite: () => ({ mutateAsync: accept, isPending: false }),
  useRejectShareInvite: () => ({ mutateAsync: reject, isPending: false }),
}))

const pending: ShareInvite = {
  id: 'i1',
  subject_type: 'category',
  subject_id: 'c1',
  subject_title: 'Reading',
  access: 'read',
  inviter_display_name: 'Ana',
  created_at: '2026-11-01T00:00:00Z',
}

describe('ShareInvitesList', () => {
  afterEach(cleanup)

  it('accepts and declines a pending invite', async () => {
    invites.list = [pending]
    render(
      <MantineProvider>
        <ShareInvitesList />
      </MantineProvider>
    )
    expect(screen.getByText(/Ana invited you to folder “Reading”/)).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Accept' }))
    expect(accept).toHaveBeenCalledWith('i1')
    await userEvent.click(screen.getByRole('button', { name: 'Decline' }))
    expect(reject).toHaveBeenCalledWith('i1')
  })

  it('leaves out declined invites and renders nothing when empty', () => {
    invites.list = [{ ...pending, rejected_at: '2026-11-01T01:00:00Z' }]
    const { container } = render(
      <MantineProvider>
        <ShareInvitesList />
      </MantineProvider>
    )
    expect(container.textContent).toBe('')
  })
})
```

Run it and confirm it fails with a missing module.

- [ ] **Step 2: Implement the list**

`components/notifications/ShareInvitesList.tsx`:

```tsx
import type { ReactElement } from 'react'
import { Button, Card, Group, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useAcceptShareInvite, useRejectShareInvite, useShareInvites } from '@/hooks'
import type { ShareInvite } from '@/schemas'

function describeInvite(invite: ShareInvite): string {
  const kind = invite.subject_type === 'dashboard' ? 'dashboard' : 'folder'
  const access = invite.access === 'write' ? 'read-write' : 'read-only'
  return `${invite.inviter_display_name} invited you to ${kind} “${invite.subject_title}” (${access})`
}

export function ShareInvitesList(): ReactElement | null {
  const list = useShareInvites()
  const accept = useAcceptShareInvite()
  const reject = useRejectShareInvite()
  const pending = (list.data?.invites ?? []).filter((i) => !i.rejected_at)

  if (pending.length === 0) return null

  async function run(action: Promise<unknown>, message: string): Promise<void> {
    try {
      await action
      notifications.show({ message })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Stack gap="xs">
      {pending.map((invite) => (
        <Card key={invite.id} withBorder padding="sm" radius="md">
          <Stack gap="xs">
            <Text size="sm">{describeInvite(invite)}</Text>
            <Group gap="xs" justify="flex-end">
              <Button
                size="compact-sm"
                variant="default"
                disabled={reject.isPending || accept.isPending}
                onClick={() => void run(reject.mutateAsync(invite.id), 'Invite declined')}
              >
                Decline
              </Button>
              <Button
                size="compact-sm"
                disabled={reject.isPending || accept.isPending}
                onClick={() => void run(accept.mutateAsync(invite.id), 'Invite accepted')}
              >
                Accept
              </Button>
            </Group>
          </Stack>
        </Card>
      ))}
    </Stack>
  )
}
```

Render `<ShareInvitesList />` in `NotificationsDrawer.tsx` directly above the first `QueryErrorAlert` line, importing it from `./ShareInvitesList`.

- [ ] **Step 3: Count invites on the bell**

In `pages/app-shell/AppShell.tsx`, add `useShareInvites` to the hooks import, and pass the header:

```tsx
  const invites = useShareInvites(!!me.data)
  const pendingInvites = invites.data?.invites.filter((i) => !i.rejected_at).length ?? 0
```

```tsx
          unreadCount={(unread.data?.count ?? 0) + pendingInvites}
```

Place the two hook lines directly after `const unread` at `AppShell.tsx:27`, and change the prop at `AppShell.tsx:80`.

- [ ] **Step 4: Add the declined card**

`pages/profile/ProfileDeclinedInvitesCard.tsx`:

```tsx
import type { ReactElement } from 'react'
import { Button, Card, Group, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useRestoreShareInvite, useShareInvites } from '@/hooks'

export function ProfileDeclinedInvitesCard(): ReactElement | null {
  const list = useShareInvites()
  const restore = useRestoreShareInvite()
  const declined = (list.data?.invites ?? []).filter((i) => !!i.rejected_at)

  if (declined.length === 0) return null

  async function onRestore(id: string): Promise<void> {
    try {
      await restore.mutateAsync(id)
      notifications.show({ message: 'Invite restored to your notifications' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Card withBorder padding="lg" radius="md">
      <Stack gap="md">
        <Title order={4}>Declined invites</Title>
        <Text size="sm" c="dimmed">
          People can't re-invite you to these until you restore them.
        </Text>
        {declined.map((invite) => (
          <Group key={invite.id} justify="space-between" wrap="nowrap">
            <Text size="sm">
              {invite.subject_title} from {invite.inviter_display_name}
            </Text>
            <Button size="compact-sm" variant="default" loading={restore.isPending} onClick={() => void onRestore(invite.id)}>
              Restore
            </Button>
          </Group>
        ))}
      </Stack>
    </Card>
  )
}
```

Render it in `ProfilePage.tsx` directly after `<ProfileBlocksCard />`.

- [ ] **Step 5: Update the share drawer message**

In `hooks/categories/useShareCategoryDrawer.ts:57` and `components/dashboard/ShareDashboardDrawer.tsx:43`, use `'Invite sent. They get access once they accept.'` for `outcome.status === 'pending'`.

- [ ] **Step 6: Verify and commit**

Run: `cd apps/web && pnpm exec prettier --write src >/dev/null; pnpm exec tsc -b 2>&1 | grep -c "error TS"; pnpm test 2>&1 | grep -E "Test Files|Tests  |FAIL"; pnpm lint 2>&1 | grep -cE "warning|error"`
Expected: `0`, all pass, `0`.

Commit: `Add accept, decline and restore to the web app.`

---

### Task 7: Smoke test and close out

**Files:** Modify `docs/ROADMAP.md`, `docs/FEATURES.md`.

- [ ] **Step 1: Smoke-test the routes**

Use the Task 11 method from `docs/superpowers/plans/2026-11-01-profile-visibility.md`: a `smoke` database in the throwaway container, the API on port 8081 with `MEILI_URL=http://127.0.0.1:1`. Assert:

| Request | Expected |
| --- | --- |
| A shares a folder with B | `status: pending` |
| B: `GET /categories/{id}` | 404 |
| B: `GET /share-invites` | 1 invite |
| Stranger: `POST /share-invites/{id}/accept` | 404 |
| B rejects, A shares again | 409 |
| B restores, then accepts | 200 |
| B: `GET /categories/{id}` | 200 |
| A's share list | B active, nothing pending |

- [ ] **Step 2: Update the docs**

Remove the "Accept / reject share flow" line from `docs/ROADMAP.md`, along with the `**Sharing**` heading if nothing is left under it. In `docs/FEATURES.md`, change the pending-invites line so it says access starts when the invitee accepts, and add after it:

```markdown
- Accept or decline shares from the notifications drawer; nothing appears in your library until you accept. Declining is silent and stops repeat invites until you restore it from Profile
```

- [ ] **Step 3: Run everything**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -cE "^crates.*(warning|error)"
cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED"
cd apps/web && pnpm exec tsc -b && pnpm lint && pnpm test && pnpm build
graphify update .
```

Expected: clippy `0`, no failed tests, clean lint, build exit 0. Remove the container afterwards.

- [ ] **Step 4: Commit**

Commit: `Record share accept and reject as shipped.`

Then ask Arsam to rebuild the dev stack and check the flow in a browser with two accounts.
