# Profile and Library Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let signed-in users view another user's profile and the bookmarks that user chose to show, with every library private until its owner opts in.

**Architecture:** A separate read path in `crates/db/src/repo/profiles.rs` computes access per request from `users.profile_visibility`, `categories.visibility`, `bookmarks.is_private`, follows and blocks. `category_access` and `bookmark_can_read` are not touched, so libraries, search and write paths cannot pick up the new tiers. The web app adds `/u/$id` and rebuilds `/profile` on the same components.

**Tech Stack:** Rust, axum, sqlx, Postgres 16; React, Mantine, TanStack Query and Router, zod, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-01-profile-visibility-design.md`

---

## Conventions for every task

- Work on `main`. Linkdean has no feature branches until its first public release.
- Rust tooling needs `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.
- DB tests skip silently when `DATABASE_URL` is unset, so a run without it proves nothing. Never point them at the dev database. Use this container:

```bash
docker rm -f linkdean-pg-check >/dev/null 2>&1
docker run -d --name linkdean-pg-check -e POSTGRES_USER=linkdean -e POSTGRES_PASSWORD=linkdean -e POSTGRES_DB=linkdean -p 127.0.0.1:55433:5432 --tmpfs /var/lib/postgresql/data postgres:16-alpine
export DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:55433/linkdean
```

- Rows store enums as strings, and API enums expose `as_str` and `parse`, matching `ShareAccess`.
- Run `graphify update .` after each task.
- End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Commit messages are one sentence ending in a period, like the existing history.

## Spec refinements

The plan narrows the spec in these places. Each one is the safer reading.

- Only categories owned by the profile's owner count towards bookmark visibility, so a third party's shared category cannot publish someone's bookmark. Deleted categories do not count.
- `display_name` on profiles falls back to `User`, not `user_share_label`, because that label can contain the email local part.
- Follower and following lists omit users the viewer could not open, so a hidden user never appears. Counts stay as raw totals.
- `ProfileView` carries `follow` only. A block relation has no use here because a blocked viewer gets a 404.
- Profile bookmarks show no custom preview image, because the preview route is gated on `bookmark_can_read`.
- `is_private` is rejected with 400 on vault bookmarks, which are never shown anyway.
- The API crate has no route test harness. Access rules are tested at the repo layer, and Task 11 smoke-tests routes with curl against a throwaway database.

## File structure

| File | Responsibility |
| --- | --- |
| `migrations/026_profile_visibility.sql` | New columns and constraints |
| `crates/core/src/models/mod.rs` | Enums, row fields, profile views |
| `crates/db/src/repo/profiles.rs` | All profile access rules and read queries |
| `crates/db/tests/profile_visibility.rs` | Access matrix |
| `crates/api/src/routes/users/mod.rs` | Four profile routes and the avatar route |
| `apps/web/src/schemas/profiles/` | zod schemas |
| `apps/web/src/hooks/profiles/` | Query hooks |
| `apps/web/src/components/profile/` | `ProfileHeader`, `ProfileBookmarksList`, `FollowListModal`, `VisibilityHint` |
| `apps/web/src/pages/user-profile/` | `/u/$id` page |
| `apps/web/src/pages/profile/ProfileEditDrawer.tsx` | Own-profile edit drawer |

---

### Task 1: Migration and models

**Files:**
- Create: `migrations/026_profile_visibility.sql`
- Modify: `crates/core/src/models/mod.rs`
- Modify: `crates/db/src/repo/users.rs` (`to_public`)
- Modify: `crates/db/src/repo/shares.rs:192` (`Category` literal)
- Modify: `crates/db/src/repo/bookmarks.rs` (both `BookmarkView` literals)
- Modify: `crates/db/tests/activity.rs:102`, `crates/db/tests/authz.rs:145,303`
- Test: `crates/db/tests/profile_visibility.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/db/tests/profile_visibility.rs`:

```rust
//! Who can see a profile and its bookmarks.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping profile_visibility tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    db::create_user(
        pool,
        &format!("{prefix}-{}@visibility.test", Uuid::new_v4()),
        Some("password-long"),
        None,
        None,
        None,
    )
    .await
    .expect("user")
}

async fn category(pool: &PgPool, owner: &User) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Visibility-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
            icon: None,
        },
        )
    .await
    .expect("category")
    .id
}

async fn bookmark(pool: &PgPool, owner: &User, category_ids: Vec<Uuid>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://visibility.example/{}", Uuid::new_v4()),
            title: Some("Bookmark".into()),
            category_ids,
            ..Default::default()
        },
        None,
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn new_rows_default_to_invisible() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let bm = bookmark(&pool, &owner, vec![cat]).await;

    assert_eq!(owner.profile_visibility, "hidden");
    let (visibility,): (String,) = sqlx::query_as("SELECT visibility FROM categories WHERE id = $1")
        .bind(cat)
        .fetch_one(&pool)
        .await
        .expect("category row");
    assert_eq!(visibility, "private");
    let (is_private,): (bool,) = sqlx::query_as("SELECT is_private FROM bookmarks WHERE id = $1")
        .bind(bm)
        .fetch_one(&pool)
        .await
        .expect("bookmark row");
    assert!(!is_private);
}

#[tokio::test]
async fn vault_category_cannot_leave_private_at_db_level() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "vault").await;
    let cat = category(&pool, &owner).await;
    sqlx::query("UPDATE categories SET is_vault = TRUE WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await
        .expect("mark vault");
    let denied = sqlx::query("UPDATE categories SET visibility = 'members' WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await;
    assert!(denied.is_err());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p linkdean-db --test profile_visibility`
Expected: compile error, no field `profile_visibility` on `User`.

- [ ] **Step 3: Add the migration**

Create `migrations/026_profile_visibility.sql`:

```sql
-- Profile and library visibility. Defaults keep every profile and library invisible.

ALTER TABLE users
    ADD COLUMN profile_visibility TEXT NOT NULL DEFAULT 'hidden'
    CONSTRAINT users_profile_visibility_check
    CHECK (profile_visibility IN ('members', 'followers', 'hidden'));

ALTER TABLE categories
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'private'
    CONSTRAINT categories_visibility_check
    CHECK (visibility IN ('private', 'followers', 'members'));

ALTER TABLE categories
    ADD CONSTRAINT categories_vault_visibility_check
    CHECK (NOT is_vault OR visibility = 'private');

ALTER TABLE bookmarks
    ADD COLUMN is_private BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX categories_owner_visibility_idx
    ON categories (owner_user_id, visibility)
    WHERE deleted_at IS NULL AND visibility <> 'private';
```

- [ ] **Step 4: Add the models**

In `crates/core/src/models/mod.rs`:

Add to `struct User`, after `has_avatar`:

```rust
    pub profile_visibility: String,
```

Add to `struct UserPublic`, after `has_avatar`:

```rust
    pub profile_visibility: String,
```

Add to `struct Category`, after `public_slug`:

```rust
    /// `private`, `followers` or `members`. Unrelated to `is_hidden`, which only affects the owner's views.
    pub visibility: String,
```

Add to `struct CategoryView`, after `is_vault`:

```rust
    pub visibility: String,
```

Add to `CategoryView::from_category`, after `is_vault: cat.is_vault,`:

```rust
            visibility: cat.visibility,
```

Add to `struct Bookmark`, after `is_vault`, and to `struct BookmarkView`, after `has_preview`:

```rust
    pub is_private: bool,
```

Add to `struct UpdateProfile`:

```rust
    #[serde(default)]
    pub profile_visibility: Option<ProfileVisibility>,
```

Add to `struct UpdateCategory`:

```rust
    #[serde(default)]
    pub visibility: Option<CategoryVisibility>,
```

Add to `struct UpdateBookmark`:

```rust
    #[serde(default)]
    pub is_private: Option<bool>,
```

Add after `ShareAccess`'s `impl` block:

```rust
/// Who can open a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileVisibility {
    Members,
    Followers,
    Hidden,
}

impl ProfileVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Members => "members",
            Self::Followers => "followers",
            Self::Hidden => "hidden",
        }
    }
}

/// Who can see a category's bookmarks on the owner's profile. "Public" is reserved for
/// the no-login Public link, so the widest tier here is `Members`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryVisibility {
    Private,
    Followers,
    Members,
}

impl CategoryVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Followers => "followers",
            Self::Members => "members",
        }
    }
}
```

Add after `struct FollowRelation`:

```rust
/// A user's profile as another signed-in user sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileView {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub has_avatar: bool,
    pub is_self: bool,
    pub follow: FollowRelation,
    /// Self only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Self only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_visibility: Option<String>,
}

/// One row of a follower or following list. Never carries an email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub id: Uuid,
    pub display_name: String,
    pub has_avatar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBookmarksPage {
    pub bookmarks: Vec<PublicBookmarkView>,
    pub total: i64,
}
```

- [ ] **Step 5: Fix the struct literals**

- `crates/db/src/repo/users.rs` `to_public`: add `profile_visibility: user.profile_visibility.clone(),` after `has_avatar`.
- `crates/db/src/repo/bookmarks.rs`: in the vault `BookmarkView` literal add `is_private: false,`. In the non-vault literal add `is_private: b.is_private,`.
- `crates/db/src/repo/shares.rs:192`: add `visibility: cat.visibility,` to the `Category` literal, using the same source variable as its other fields.
- `crates/db/tests/activity.rs:102`, `crates/db/tests/authz.rs:145`, `crates/db/tests/authz.rs:303`: add `visibility: None,` to each `UpdateCategory` literal.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p linkdean-db --test profile_visibility && cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED"`
Expected: both new tests pass and no test fails anywhere.

- [ ] **Step 7: Commit**

```bash
git add migrations crates
git commit -m "Add the profile and library visibility columns."
```

---

### Task 2: Profile access

**Files:**
- Create: `crates/db/src/repo/profiles.rs`
- Modify: `crates/db/src/repo/mod.rs`
- Test: `crates/db/tests/profile_visibility.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/db/tests/profile_visibility.rs`, and add `Error` to the `linkdean_core` import:

```rust
async fn set_profile(pool: &PgPool, user: &User, visibility: &str) {
    sqlx::query("UPDATE users SET profile_visibility = $2 WHERE id = $1")
        .bind(user.id)
        .bind(visibility)
        .execute(pool)
        .await
        .expect("set profile visibility");
}

async fn can_open(pool: &PgPool, viewer: &User, target: &User) -> bool {
    match db::get_profile(pool, viewer.id, target.id).await {
        Ok(_) => true,
        Err(Error::NotFound) => false,
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[tokio::test]
async fn profile_access_matrix() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let cases = [
        ("members", false, true),
        ("members", true, true),
        ("followers", false, false),
        ("followers", true, true),
        ("hidden", false, false),
        ("hidden", true, false),
    ];
    for (setting, follows, expected) in cases {
        let target = user(&pool, "target").await;
        let viewer = user(&pool, "viewer").await;
        set_profile(&pool, &target, setting).await;
        if follows {
            db::follow_user(&pool, viewer.id, target.id).await.expect("follow");
        }
        assert_eq!(
            can_open(&pool, &viewer, &target).await,
            expected,
            "{setting}, follows: {follows}"
        );
        assert!(can_open(&pool, &target, &target).await, "self, {setting}");
    }
}

#[tokio::test]
async fn blocked_users_get_not_found_in_both_directions() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a").await;
    let b = user(&pool, "b").await;
    set_profile(&pool, &a, "members").await;
    set_profile(&pool, &b, "members").await;
    db::block_user(&pool, a.id, b.id).await.expect("block");
    assert!(!can_open(&pool, &a, &b).await);
    assert!(!can_open(&pool, &b, &a).await);
}

#[tokio::test]
async fn unknown_user_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let viewer = user(&pool, "viewer").await;
    let missing = db::get_profile(&pool, viewer.id, Uuid::new_v4()).await;
    assert!(matches!(missing, Err(Error::NotFound)));
}

#[tokio::test]
async fn email_and_setting_are_self_only() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;

    let own = db::get_profile(&pool, target.id, target.id).await.expect("own");
    assert!(own.is_self);
    assert_eq!(own.email.as_deref(), Some(target.email.as_str()));
    assert_eq!(own.profile_visibility.as_deref(), Some("members"));

    let other = db::get_profile(&pool, viewer.id, target.id).await.expect("other");
    assert!(!other.is_self);
    assert!(other.email.is_none());
    assert!(other.profile_visibility.is_none());
    assert!(!other.display_name.contains('@'));
    assert_eq!(other.display_name, "User");
}

#[tokio::test]
async fn losing_the_follow_loses_access_immediately() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "followers").await;
    db::follow_user(&pool, viewer.id, target.id).await.expect("follow");
    assert!(can_open(&pool, &viewer, &target).await);
    db::unfollow_user(&pool, viewer.id, target.id).await.expect("unfollow");
    assert!(!can_open(&pool, &viewer, &target).await);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p linkdean-db --test profile_visibility`
Expected: compile error, `get_profile` not found in `linkdean_db`.

- [ ] **Step 3: Implement**

Create `crates/db/src/repo/profiles.rs`:

```rust
//! Profile access. Visibility is computed per request and is separate from
//! `category_access`, which only covers ownership and explicit shares.

use linkdean_core::{Error, ProfileView, Result, User};
use sqlx::PgPool;
use uuid::Uuid;

use super::blocks::is_blocked_either_way;
use super::follows::{follow_relation, is_following};
use super::users::find_user_by_id;

/// Display name without the email local part that `user_share_label` can fall back to.
fn profile_label(user: &User) -> String {
    user.display_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("User")
        .to_string()
}

async fn can_view_profile(pool: &PgPool, viewer_id: Uuid, target: &User) -> Result<bool> {
    if viewer_id == target.id {
        return Ok(true);
    }
    if is_blocked_either_way(pool, viewer_id, target.id).await? {
        return Ok(false);
    }
    match target.profile_visibility.as_str() {
        "members" => Ok(true),
        "followers" => is_following(pool, viewer_id, target.id).await,
        _ => Ok(false),
    }
}

/// The target user, or `NotFound` for every denial so a hidden user's existence never shows.
pub async fn require_viewable_profile(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<User> {
    let target = find_user_by_id(pool, target_id)
        .await?
        .ok_or(Error::NotFound)?;
    if !can_view_profile(pool, viewer_id, &target).await? {
        return Err(Error::NotFound);
    }
    Ok(target)
}

pub async fn get_profile(pool: &PgPool, viewer_id: Uuid, target_id: Uuid) -> Result<ProfileView> {
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    let is_self = viewer_id == target.id;
    Ok(ProfileView {
        id: target.id,
        display_name: profile_label(&target),
        bio: target.bio.clone(),
        has_avatar: target.has_avatar,
        is_self,
        follow: follow_relation(pool, viewer_id, target.id).await?,
        email: is_self.then(|| target.email.clone()),
        profile_visibility: is_self.then(|| target.profile_visibility.clone()),
    })
}
```

In `crates/db/src/repo/mod.rs` add `mod profiles;` after `mod platform;` and `pub use profiles::*;` after `pub use platform::*;`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p linkdean-db --test profile_visibility`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "Add profile access rules."
```

---

### Task 3: Visible bookmarks

**Files:**
- Modify: `crates/db/src/repo/categories.rs:894-907`
- Modify: `crates/db/src/repo/profiles.rs`
- Test: `crates/db/tests/profile_visibility.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/db/tests/profile_visibility.rs`:

```rust
async fn set_category(pool: &PgPool, id: Uuid, visibility: &str) {
    sqlx::query("UPDATE categories SET visibility = $2 WHERE id = $1")
        .bind(id)
        .bind(visibility)
        .execute(pool)
        .await
        .expect("set category visibility");
}

async fn visible_ids(pool: &PgPool, viewer: &User, target: &User) -> Vec<Uuid> {
    db::profile_visible_bookmarks(pool, viewer.id, target.id, None, None)
        .await
        .expect("visible bookmarks")
        .bookmarks
        .into_iter()
        .map(|b| b.id)
        .collect()
}

#[tokio::test]
async fn category_tier_matrix() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let cases = [
        ("private", false, false),
        ("private", true, false),
        ("followers", false, false),
        ("followers", true, true),
        ("members", false, true),
        ("members", true, true),
    ];
    for (tier, follows, expected) in cases {
        let target = user(&pool, "target").await;
        let viewer = user(&pool, "viewer").await;
        set_profile(&pool, &target, "members").await;
        let cat = category(&pool, &target).await;
        set_category(&pool, cat, tier).await;
        let bm = bookmark(&pool, &target, vec![cat]).await;
        if follows {
            db::follow_user(&pool, viewer.id, target.id).await.expect("follow");
        }
        let ids = visible_ids(&pool, &viewer, &target).await;
        assert_eq!(ids.contains(&bm), expected, "{tier}, follows: {follows}");
    }
}

#[tokio::test]
async fn excluded_bookmarks_stay_excluded() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;
    let cat = category(&pool, &target).await;
    set_category(&pool, cat, "members").await;

    let shown = bookmark(&pool, &target, vec![cat]).await;
    let private = bookmark(&pool, &target, vec![cat]).await;
    sqlx::query("UPDATE bookmarks SET is_private = TRUE WHERE id = $1")
        .bind(private)
        .execute(&pool)
        .await
        .expect("mark private");
    let vault = bookmark(&pool, &target, vec![cat]).await;
    sqlx::query("UPDATE bookmarks SET is_vault = TRUE WHERE id = $1")
        .bind(vault)
        .execute(&pool)
        .await
        .expect("mark vault");
    let trashed = bookmark(&pool, &target, vec![cat]).await;
    db::delete_bookmark(&pool, target.id, trashed).await.expect("trash");
    let uncategorized = bookmark(&pool, &target, vec![]).await;

    let ids = visible_ids(&pool, &viewer, &target).await;
    assert_eq!(ids, vec![shown], "uncategorized {uncategorized} must not show");
}

#[tokio::test]
async fn any_allowing_category_is_enough() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;
    let open = category(&pool, &target).await;
    let closed = category(&pool, &target).await;
    set_category(&pool, open, "members").await;
    let bm = bookmark(&pool, &target, vec![closed, open]).await;
    assert_eq!(visible_ids(&pool, &viewer, &target).await, vec![bm]);
}

#[tokio::test]
async fn a_third_partys_category_does_not_publish_a_bookmark() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let third = user(&pool, "third").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;
    let theirs = category(&pool, &third).await;
    set_category(&pool, theirs, "members").await;
    let bm = bookmark(&pool, &target, vec![]).await;
    sqlx::query("INSERT INTO bookmark_categories (bookmark_id, category_id) VALUES ($1, $2)")
        .bind(bm)
        .bind(theirs)
        .execute(&pool)
        .await
        .expect("link");
    assert!(visible_ids(&pool, &viewer, &target).await.is_empty());
}

#[tokio::test]
async fn hidden_profile_hides_bookmarks_and_self_sees_preview() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    let cat = category(&pool, &target).await;
    set_category(&pool, cat, "followers").await;
    let bm = bookmark(&pool, &target, vec![cat]).await;

    let denied = db::profile_visible_bookmarks(&pool, viewer.id, target.id, None, None).await;
    assert!(matches!(denied, Err(Error::NotFound)));
    assert_eq!(visible_ids(&pool, &target, &target).await, vec![bm]);
}

#[tokio::test]
async fn paging_reports_the_total() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let cat = category(&pool, &target).await;
    set_category(&pool, cat, "members").await;
    for _ in 0..3 {
        bookmark(&pool, &target, vec![cat]).await;
    }
    let page = db::profile_visible_bookmarks(&pool, target.id, target.id, Some(2), Some(2))
        .await
        .expect("page");
    assert_eq!(page.total, 3);
    assert_eq!(page.bookmarks.len(), 1);

    for (limit, offset) in [(Some(0), None), (Some(101), None), (None, Some(-1))] {
        let bad = db::profile_visible_bookmarks(&pool, target.id, target.id, limit, offset).await;
        assert!(matches!(bad, Err(Error::BadRequest(_))));
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p linkdean-db --test profile_visibility`
Expected: compile error, `profile_visible_bookmarks` not found.

- [ ] **Step 3: Share the view conversion**

In `crates/db/src/repo/categories.rs`, replace the body of the `for b in rows` loop in `get_public_list_by_slug` with:

```rust
        bookmarks.push(public_bookmark_view(pool, b).await?);
```

Add above `get_public_list_by_slug`:

```rust
/// Bookmark fields that are safe to show to people the owner did not share with directly.
pub(super) async fn public_bookmark_view(
    pool: &PgPool,
    b: linkdean_core::Bookmark,
) -> Result<PublicBookmarkView> {
    let tags = tags_for_bookmark(pool, b.id).await?;
    Ok(PublicBookmarkView {
        id: b.id,
        url: b.url,
        title: b.title,
        description: b.description,
        favicon_url: b.favicon_url,
        preview_image_url: b.preview_image_url,
        has_preview: b.has_preview,
        tags,
    })
}
```

- [ ] **Step 4: Implement the query**

In `crates/db/src/repo/profiles.rs`, extend the imports:

```rust
use linkdean_core::{Bookmark, Error, ProfileBookmarksPage, ProfileView, Result, User};

use super::categories::public_bookmark_view;
```

Add:

```rust
const DEFAULT_PAGE_SIZE: i64 = 50;
const MAX_PAGE_SIZE: i64 = 100;

/// `$1` owner, `$2` category tiers the viewer may see. Only the owner's own categories
/// count, so a category shared by someone else cannot publish a bookmark.
const VISIBLE_BOOKMARK_FILTER: &str = r#"
    b.owner_user_id = $1
    AND b.deleted_at IS NULL
    AND b.is_vault = FALSE
    AND b.is_private = FALSE
    AND EXISTS (
        SELECT 1 FROM categories c
        WHERE c.owner_user_id = $1
          AND c.deleted_at IS NULL
          AND c.visibility = ANY($2)
          AND (
            c.id = b.category_id
            OR EXISTS (
                SELECT 1 FROM bookmark_categories bc
                WHERE bc.bookmark_id = b.id AND bc.category_id = c.id
            )
          )
    )
"#;

fn page_bounds(limit: Option<i64>, offset: Option<i64>) -> Result<(i64, i64)> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE);
    let offset = offset.unwrap_or(0);
    if !(1..=MAX_PAGE_SIZE).contains(&limit) {
        return Err(Error::BadRequest(format!(
            "limit must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    if offset < 0 {
        return Err(Error::BadRequest("offset must not be negative".into()));
    }
    Ok((limit, offset))
}

/// The owner previews everything others could see, so self counts as a follower.
async fn visible_tiers(pool: &PgPool, viewer_id: Uuid, target_id: Uuid) -> Result<Vec<String>> {
    let follower = viewer_id == target_id || is_following(pool, viewer_id, target_id).await?;
    let mut tiers = vec!["members".to_string()];
    if follower {
        tiers.push("followers".to_string());
    }
    Ok(tiers)
}

pub async fn profile_visible_bookmarks(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ProfileBookmarksPage> {
    let (limit, offset) = page_bounds(limit, offset)?;
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    let tiers = visible_tiers(pool, viewer_id, target.id).await?;

    let count_sql = format!("SELECT COUNT(*) FROM bookmarks b WHERE {VISIBLE_BOOKMARK_FILTER}");
    let (total,): (i64,) = sqlx::query_as(&count_sql)
        .bind(target.id)
        .bind(&tiers)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let rows_sql = format!(
        "SELECT b.* FROM bookmarks b WHERE {VISIBLE_BOOKMARK_FILTER} \
         ORDER BY b.updated_at DESC, b.id LIMIT $3 OFFSET $4"
    );
    let rows = sqlx::query_as::<_, Bookmark>(&rows_sql)
        .bind(target.id)
        .bind(&tiers)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let mut bookmarks = Vec::with_capacity(rows.len());
    for b in rows {
        bookmarks.push(public_bookmark_view(pool, b).await?);
    }
    Ok(ProfileBookmarksPage { bookmarks, total })
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p linkdean-db --test profile_visibility && cargo test -p linkdean-db --test dashboards`
Expected: all pass. `bookmark_categories` may have extra required columns; if the third-party test's insert fails, read the table in `migrations/` and supply them.

- [ ] **Step 6: Commit**

```bash
git add crates
git commit -m "Add the bookmark read path for profiles."
```

---

### Task 4: Follower lists

**Files:**
- Modify: `crates/db/src/repo/profiles.rs`
- Test: `crates/db/tests/profile_visibility.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn follow_lists_share_the_profile_gate_and_hide_unopenable_users() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    let open_fan = user(&pool, "open").await;
    let hidden_fan = user(&pool, "hidden").await;
    set_profile(&pool, &open_fan, "members").await;
    db::follow_user(&pool, open_fan.id, target.id).await.expect("follow");
    db::follow_user(&pool, hidden_fan.id, target.id).await.expect("follow");

    let denied = db::profile_followers(&pool, viewer.id, target.id).await;
    assert!(matches!(denied, Err(Error::NotFound)));

    set_profile(&pool, &target, "members").await;
    let ids: Vec<Uuid> = db::profile_followers(&pool, viewer.id, target.id)
        .await
        .expect("followers")
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert_eq!(ids, vec![open_fan.id]);

    let following: Vec<Uuid> = db::profile_following(&pool, viewer.id, open_fan.id)
        .await
        .expect("following")
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert_eq!(following, vec![target.id]);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p linkdean-db --test profile_visibility follow_lists`
Expected: compile error, `profile_followers` not found.

- [ ] **Step 3: Implement**

In `crates/db/src/repo/profiles.rs`, add `FollowUserView` and `ProfileSummary` to the `linkdean_core` import, change the follows import to `use super::follows::{follow_relation, is_following, list_followers, list_following};`, and add:

```rust
async fn visible_summaries(
    pool: &PgPool,
    viewer_id: Uuid,
    rows: Vec<FollowUserView>,
) -> Result<Vec<ProfileSummary>> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(user) = find_user_by_id(pool, row.id).await? else {
            continue;
        };
        if can_view_profile(pool, viewer_id, &user).await? {
            out.push(ProfileSummary {
                id: user.id,
                display_name: profile_label(&user),
                has_avatar: user.has_avatar,
            });
        }
    }
    Ok(out)
}

pub async fn profile_followers(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<Vec<ProfileSummary>> {
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    visible_summaries(pool, viewer_id, list_followers(pool, target.id).await?).await
}

pub async fn profile_following(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<Vec<ProfileSummary>> {
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    visible_summaries(pool, viewer_id, list_following(pool, target.id).await?).await
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p linkdean-db --test profile_visibility`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "Add follower and following lists for profiles."
```

---

### Task 5: Write paths

**Files:**
- Modify: `crates/db/src/repo/users.rs` (`update_profile`)
- Modify: `crates/db/src/repo/categories.rs` (`update_category`)
- Modify: `crates/db/src/repo/bookmarks.rs` (`update_has_content`, `update_bookmark`)
- Test: `crates/db/tests/profile_visibility.rs`

- [ ] **Step 1: Write the failing test**

Add `CategoryVisibility`, `ProfileVisibility`, `UpdateBookmark`, `UpdateCategory`, `UpdateProfile`, `CreateCategoryShare` and `ShareAccess` to the import. Read `CreateCategoryShare` in `crates/core/src/models/mod.rs` first and match its fields; the helper `share` in `crates/db/tests/blocks.rs` shows the call.

```rust
fn visibility_patch(visibility: CategoryVisibility) -> UpdateCategory {
    UpdateCategory {
        name: None,
        parent_id: None,
        is_hidden: None,
        position: None,
        icon: None,
        visibility: Some(visibility),
    }
}

#[tokio::test]
async fn owner_sets_profile_and_category_visibility() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let updated = db::update_profile(
        &pool,
        owner.id,
        &UpdateProfile {
            display_name: None,
            bio: None,
            timezone: None,
            email: None,
            profile_visibility: Some(ProfileVisibility::Followers),
        },
    )
    .await
    .expect("update profile");
    assert_eq!(updated.profile_visibility, "followers");

    let cat = category(&pool, &owner).await;
    let view = db::update_category(&pool, owner.id, cat, visibility_patch(CategoryVisibility::Members))
        .await
        .expect("update category");
    assert_eq!(view.visibility, "members");

    let events = db::list_activity(&pool, owner.id, Default::default())
        .await
        .expect("activity");
    assert!(events.iter().any(|e| e.subject_id == cat && e.summary.contains("Members")));
}

#[tokio::test]
async fn vault_category_rejects_wider_visibility() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "vault").await;
    let cat = category(&pool, &owner).await;
    sqlx::query("UPDATE categories SET is_vault = TRUE WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await
        .expect("mark vault");
    let denied =
        db::update_category(&pool, owner.id, cat, visibility_patch(CategoryVisibility::Members)).await;
    assert!(matches!(denied, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn only_the_owner_sets_visibility() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let writer = user(&pool, "writer").await;
    let cat = category(&pool, &owner).await;
    let bm = bookmark(&pool, &owner, vec![cat]).await;
    db::create_share(
        &pool,
        owner.id,
        cat,
        CreateCategoryShare {
            email: writer.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("share");

    let denied =
        db::update_category(&pool, writer.id, cat, visibility_patch(CategoryVisibility::Members)).await;
    assert!(matches!(denied, Err(Error::Forbidden)));

    let patch = UpdateBookmark {
        is_private: Some(true),
        ..Default::default()
    };
    let denied = db::update_bookmark(&pool, writer.id, bm, patch, None).await;
    assert!(matches!(denied, Err(Error::Forbidden)));
}

#[tokio::test]
async fn is_private_round_trips_and_is_rejected_on_vault() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let bm = bookmark(&pool, &owner, vec![cat]).await;
    let patch = UpdateBookmark {
        is_private: Some(true),
        ..Default::default()
    };
    let view = db::update_bookmark(&pool, owner.id, bm, patch.clone(), None)
        .await
        .expect("set private");
    assert!(view.is_private);

    sqlx::query("UPDATE bookmarks SET is_vault = TRUE WHERE id = $1")
        .bind(bm)
        .execute(&pool)
        .await
        .expect("mark vault");
    let denied = db::update_bookmark(&pool, owner.id, bm, patch, None).await;
    assert!(matches!(denied, Err(Error::BadRequest(_))));
}
```

Check `db::list_activity`'s real signature at `crates/db/src/repo/activity.rs:167` before running, and adjust that call and the field names to match.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p linkdean-db --test profile_visibility`
Expected: tests fail. `profile_visibility`, `visibility` and `is_private` patches are ignored or rejected as empty.

- [ ] **Step 3: Implement `update_profile`**

In `crates/db/src/repo/users.rs`, before the `UPDATE`:

```rust
    let profile_visibility = patch
        .profile_visibility
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| user.profile_visibility.clone());
```

Add `profile_visibility = $6,` after `email = $5,` in the SQL, and `.bind(profile_visibility)` after `.bind(email)`.

- [ ] **Step 4: Implement `update_category`**

In `crates/db/src/repo/categories.rs`, add `let before_visibility = cat.visibility.clone();` beside the other `before_*` values, and after the `is_hidden` block:

```rust
    if let Some(visibility) = req.visibility {
        if !access.is_owner() {
            return Err(Error::Forbidden);
        }
        if cat.is_vault && visibility != linkdean_core::CategoryVisibility::Private {
            return Err(Error::BadRequest(
                "vault categories must stay private".into(),
            ));
        }
        cat.visibility = visibility.as_str().to_string();
    }
```

After the `is_hidden` change block:

```rust
    if cat.visibility != before_visibility {
        changes.push(format!("visibility set to {}", visibility_label(&cat.visibility)));
        change_keys.push("visibility");
    }
```

Add a helper above `update_category`:

```rust
fn visibility_label(visibility: &str) -> &'static str {
    match visibility {
        "followers" => "Followers",
        "members" => "Members",
        _ => "Only me",
    }
}
```

In the `UPDATE categories` SQL add `visibility = $7,` after `icon = $5,`, and `.bind(&cat.visibility)` after `.bind(id)`.

- [ ] **Step 5: Implement `is_private`**

In `crates/db/src/repo/bookmarks.rs`, add to `update_has_content`:

```rust
        || req.is_private.is_some()
```

Add above `update_bookmark`:

```rust
/// Owner-only. Vault bookmarks never appear on a profile, so the flag does not apply to them.
async fn set_bookmark_private(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    is_private: bool,
) -> Result<()> {
    let existing = get_bookmark_raw(pool, user_id, id).await?;
    if existing.owner_user_id != user_id {
        return Err(Error::Forbidden);
    }
    if existing.is_vault {
        return Err(Error::BadRequest(
            "vault bookmarks cannot be marked private".into(),
        ));
    }
    if existing.is_private == is_private {
        return Ok(());
    }
    sqlx::query("UPDATE bookmarks SET is_private = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(is_private)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let change = if is_private {
        "set to Only me"
    } else {
        "visible through its categories"
    };
    let summary = super::activity::update_summary(
        linkdean_core::ActivitySubjectType::Bookmark,
        &existing.title,
        &[change.to_string()],
    );
    super::activity::record_activity_with(
        pool,
        user_id,
        linkdean_core::ActivityAction::Updated,
        super::activity::ActivitySubject {
            subject_type: linkdean_core::ActivitySubjectType::Bookmark,
            subject_id: id,
            subject_title: &existing.title,
        },
        Some(&summary),
        serde_json::json!({ "changes": ["is_private"] }),
    )
    .await;
    Ok(())
}
```

In `update_bookmark`, directly after the permission checks and before `if flags {`:

```rust
    if let Some(is_private) = req.is_private {
        set_bookmark_private(pool, user_id, id, is_private).await?;
    }
```

- [ ] **Step 6: Run the tests**

Run: `cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: no failures.

- [ ] **Step 7: Commit**

```bash
git add crates
git commit -m "Let owners set profile, category and bookmark visibility."
```

---

### Task 6: Routes

**Files:**
- Modify: `crates/api/src/routes/users/mod.rs`

- [ ] **Step 1: Add the routes**

In `router()`, after the `/users/{id}/mute-status` route:

```rust
        .route("/users/{id}/profile", get(user_profile))
        .route("/users/{id}/bookmarks", get(user_bookmarks))
        .route("/users/{id}/followers", get(user_followers))
        .route("/users/{id}/following", get(user_following))
        .route("/users/{id}/avatar", get(user_avatar))
```

Change the extract import to `use axum::extract::{Multipart, Path, Query, State};`, add `ProfileBookmarksPage` and `ProfileView` to the `linkdean_core` import, and add `use serde::Deserialize;`.

Change `get_avatar` so the file reading is shared:

```rust
async fn get_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<impl IntoResponse> {
    avatar_response(&state, user.id, user.has_avatar).await
}

async fn avatar_response(
    state: &AppState,
    user_id: Uuid,
    has_avatar: bool,
) -> ApiResult<impl IntoResponse> {
    if !has_avatar {
        return Err(Error::NotFound.into());
    }
    let path = state.avatar_path(user_id);
    let bytes = tokio::fs::read(&path).await.map_err(|_| Error::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    Ok((StatusCode::OK, headers, bytes))
}
```

Add the handlers:

```rust
#[derive(Debug, Deserialize)]
struct ProfileBookmarksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn user_profile(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProfileView>> {
    Ok(Json(db::get_profile(&state.pool, user.id, id).await?))
}

async fn user_bookmarks(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<ProfileBookmarksQuery>,
) -> ApiResult<Json<ProfileBookmarksPage>> {
    let page =
        db::profile_visible_bookmarks(&state.pool, user.id, id, query.limit, query.offset).await?;
    Ok(Json(page))
}

async fn user_followers(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let users = db::profile_followers(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "users": users })))
}

async fn user_following(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let users = db::profile_following(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "users": users })))
}

async fn user_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let target = db::require_viewable_profile(&state.pool, user.id, id).await?;
    avatar_response(&state, target.id, target.has_avatar).await
}
```

- [ ] **Step 2: Check it builds cleanly**

Run: `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -cE "^crates.*(warning|error)"`
Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add crates
git commit -m "Add the profile API routes."
```

---

### Task 7: Web data layer

**Files:**
- Create: `apps/web/src/schemas/profiles/profiles.ts`, `apps/web/src/schemas/profiles/index.ts`
- Create: `apps/web/src/hooks/profiles/useProfiles.ts`, `apps/web/src/hooks/profiles/index.ts`
- Modify: `apps/web/src/schemas/index.ts`, `apps/web/src/hooks/index.ts`, `apps/web/src/api/api.ts`
- Modify: `apps/web/src/schemas/auth/auth.ts`, `apps/web/src/schemas/categories/categories.ts`, `apps/web/src/schemas/bookmarks/bookmarks.ts`
- Modify: `apps/web/src/hooks/settings/useSettings.ts`, `apps/web/src/hooks/avatar/useAvatarImage.ts`
- Modify: `apps/web/src/hooks/categories/useCategories.ts` if its mutation body type lists fields

- [ ] **Step 1: Add the schemas**

Create `apps/web/src/schemas/profiles/profiles.ts`:

```ts
import { z } from 'zod'
import { publicBookmarkSchema } from '../categories'
import { followRelationSchema } from '../follows'

export const profileVisibilitySchema = z.enum(['members', 'followers', 'hidden'])
export const categoryVisibilitySchema = z.enum(['private', 'followers', 'members'])

export const profileSchema = z.object({
  id: z.string(),
  display_name: z.string(),
  bio: z.string().nullish(),
  has_avatar: z.boolean(),
  is_self: z.boolean(),
  follow: followRelationSchema,
  email: z.string().nullish(),
  profile_visibility: profileVisibilitySchema.nullish(),
})

export const profileSummarySchema = z.object({
  id: z.string(),
  display_name: z.string(),
  has_avatar: z.boolean(),
})

export const profileSummariesEnvelopeSchema = z.object({
  users: z.array(profileSummarySchema),
})

export const profileBookmarksPageSchema = z.object({
  bookmarks: z.array(publicBookmarkSchema),
  total: z.number(),
})

export type ProfileVisibility = z.infer<typeof profileVisibilitySchema>
export type CategoryVisibility = z.infer<typeof categoryVisibilitySchema>
export type Profile = z.infer<typeof profileSchema>
export type ProfileSummary = z.infer<typeof profileSummarySchema>
export type ProfileBookmarksPage = z.infer<typeof profileBookmarksPageSchema>
```

Create `apps/web/src/schemas/profiles/index.ts` with `export * from './profiles'`. Add `export * from './profiles'` to `schemas/index.ts` after `./notifications`. If `publicBookmarkSchema` is not exported from `schemas/categories/categories.ts`, add `export` to it.

Add to `userPublicSchema` in `schemas/auth/auth.ts`:

```ts
  profile_visibility: z.enum(['members', 'followers', 'hidden']).default('hidden'),
```

Add to the category schema in `schemas/categories/categories.ts`, after `is_hidden`:

```ts
  visibility: z.enum(['private', 'followers', 'members']).default('private'),
```

Add to `bookmarkSchema` in `schemas/bookmarks/bookmarks.ts`, after `is_favorite`:

```ts
  is_private: z.boolean().default(false),
```

These use literal enums because `profiles.ts` imports from `categories`, and importing back would make a cycle.

- [ ] **Step 2: Add the API calls**

In `api/api.ts`, extend the imports from the schemas module with `profileSchema`, `profileBookmarksPageSchema`, `profileSummariesEnvelopeSchema` and the types `Profile`, `ProfileBookmarksPage`, `ProfileSummary`, `ProfileVisibility`, `CategoryVisibility`.

Add `profile_visibility?: ProfileVisibility` to the `updateProfile` body, `visibility: CategoryVisibility` to the `updateCategory` body, and `is_private: boolean` to the `updateBookmark` body. Add `profile_visibility?: ProfileVisibility` to `ProfileBody` in `hooks/settings/useSettings.ts`, importing the type from `@/schemas`. If `useUpdateCategory` in `hooks/categories/useCategories.ts` lists body fields, add `visibility` there too.

After `followStatus`:

```ts
  userProfile: (id: string): Promise<Profile> => req(`/users/${id}/profile`, {}, profileSchema),

  userBookmarks: (id: string, limit: number, offset: number): Promise<ProfileBookmarksPage> =>
    req(`/users/${id}/bookmarks?limit=${limit}&offset=${offset}`, {}, profileBookmarksPageSchema),

  userFollowers: (id: string): Promise<{ users: ProfileSummary[] }> =>
    req(`/users/${id}/followers`, {}, profileSummariesEnvelopeSchema),

  userFollowing: (id: string): Promise<{ users: ProfileSummary[] }> =>
    req(`/users/${id}/following`, {}, profileSummariesEnvelopeSchema),
```

- [ ] **Step 3: Add the hooks**

Create `apps/web/src/hooks/profiles/useProfiles.ts`:

```ts
import { keepPreviousData, useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { Profile, ProfileBookmarksPage, ProfileSummary } from '@/schemas'

export const PROFILE_PAGE_SIZE = 24

export function useProfile(userId: string): UseQueryResult<Profile> {
  return useQuery({
    queryKey: ['profiles', userId],
    queryFn: () => api.userProfile(userId),
    retry: false,
  })
}

export function useProfileBookmarks(userId: string, page: number): UseQueryResult<ProfileBookmarksPage> {
  return useQuery({
    queryKey: ['profiles', userId, 'bookmarks', page],
    queryFn: () => api.userBookmarks(userId, PROFILE_PAGE_SIZE, (page - 1) * PROFILE_PAGE_SIZE),
    placeholderData: keepPreviousData,
    retry: false,
  })
}

export function useProfileFollows(
  userId: string,
  kind: 'followers' | 'following',
  enabled: boolean
): UseQueryResult<{ users: ProfileSummary[] }> {
  return useQuery({
    queryKey: ['profiles', userId, kind],
    queryFn: () => (kind === 'followers' ? api.userFollowers(userId) : api.userFollowing(userId)),
    enabled,
    retry: false,
  })
}
```

Create `apps/web/src/hooks/profiles/index.ts`:

```ts
export { PROFILE_PAGE_SIZE, useProfile, useProfileBookmarks, useProfileFollows } from './useProfiles'
```

Add `export * from './profiles'` to `hooks/index.ts` after `./prefs`.

In `hooks/follows/useFollows.ts`, inside `invalidateFollowQueries`, add:

```ts
  void qc.invalidateQueries({ queryKey: ['profiles'] })
```

In `hooks/settings/useSettings.ts`, inside `useUpdateProfile`'s `onSuccess`, add the same line.

- [ ] **Step 4: Let avatars load for other users**

Change `hooks/avatar/useAvatarImage.ts`:

```ts
export function useAvatarImage(hasAvatar: boolean, bust?: number | string, userId = 'me'): string | undefined {
```

Change the fetch to `` fetch(`/api/users/${userId}/avatar${q}`, { ``, and add `userId` to the effect deps: `[hasAvatar, bust, userId]`.

- [ ] **Step 5: Check it compiles**

Run: `cd apps/web && pnpm exec tsc -b 2>&1 | grep -c "error TS"`
Expected: `0`.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src
git commit -m "Add the web data layer for profiles."
```

---

### Task 8: Profile components and the `/u/$id` page

**Files:**
- Create: `apps/web/src/components/profile/ProfileHeader.tsx`, `ProfileBookmarksList.tsx`, `FollowListModal.tsx`, `index.ts`
- Create: `apps/web/src/components/profile/ProfileHeader.test.tsx`
- Create: `apps/web/src/pages/user-profile/UserProfilePage.tsx`, `index.ts`, `UserProfilePage.test.tsx`
- Create: `apps/web/src/routes/_app/u.$id.tsx`
- Modify: `apps/web/src/components/index.ts`, `apps/web/src/pages/index.ts`

- [ ] **Step 1: Write the failing tests**

Create `apps/web/src/components/profile/ProfileHeader.test.tsx`:

```tsx
import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Profile } from '@/schemas'
import { ProfileHeader } from './ProfileHeader'

vi.mock('@/hooks', () => ({ useAvatarImage: () => undefined }))

const profile: Profile = {
  id: 'u1',
  display_name: 'Ana',
  bio: 'Reads a lot',
  has_avatar: false,
  is_self: false,
  follow: { following: false, followers: 3, following_count: 1 },
}

function renderHeader(ui: ReactElement): void {
  render(<MantineProvider>{ui}</MantineProvider>)
}

describe('ProfileHeader', () => {
  afterEach(cleanup)

  it('offers follow, not edit, on someone else', async () => {
    const onToggleFollow = vi.fn()
    renderHeader(
      <ProfileHeader profile={profile} onToggleFollow={onToggleFollow} onEdit={vi.fn()} onOpenList={vi.fn()} />
    )
    expect(screen.queryByRole('button', { name: /edit profile/i })).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Follow' }))
    expect(onToggleFollow).toHaveBeenCalled()
  })

  it('shows email and edit on your own profile', () => {
    renderHeader(
      <ProfileHeader
        profile={{ ...profile, is_self: true, email: 'ana@example.com' }}
        onToggleFollow={vi.fn()}
        onEdit={vi.fn()}
        onOpenList={vi.fn()}
      />
    )
    expect(screen.getByText('ana@example.com')).toBeTruthy()
    expect(screen.getByRole('button', { name: /edit profile/i })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Follow' })).toBeNull()
  })

  it('opens the follower list from the count', async () => {
    const onOpenList = vi.fn()
    renderHeader(
      <ProfileHeader profile={profile} onToggleFollow={vi.fn()} onEdit={vi.fn()} onOpenList={onOpenList} />
    )
    await userEvent.click(screen.getByRole('button', { name: '3 followers' }))
    expect(onOpenList).toHaveBeenCalledWith('followers')
  })
})
```

Create `apps/web/src/pages/user-profile/UserProfilePage.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import { UserProfileView } from './UserProfilePage'

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api: { ...actual.api, userProfile: vi.fn(), userBookmarks: vi.fn() } }
})

describe('UserProfileView', () => {
  afterEach(cleanup)

  it('shows the unavailable state on a denied profile', async () => {
    vi.mocked(api.userProfile).mockRejectedValue(new Error('not found'))
    render(
      <MantineProvider>
        <QueryClientProvider client={new QueryClient()}>
          <UserProfileView userId="u1" onEdit={vi.fn()} />
        </QueryClientProvider>
      </MantineProvider>
    )
    expect(await screen.findByText('Profile not available')).toBeTruthy()
  })
})
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd apps/web && pnpm exec vitest run src/components/profile src/pages/user-profile`
Expected: FAIL, modules not found.

- [ ] **Step 3: Implement the components**

Create `apps/web/src/components/profile/ProfileHeader.tsx`:

```tsx
import type { ReactElement } from 'react'
import { Avatar, Button, Group, Stack, Text, Title, UnstyledButton } from '@mantine/core'
import { useAvatarImage } from '@/hooks'
import type { Profile } from '@/schemas'

export type FollowListKind = 'followers' | 'following'

type Props = {
  profile: Profile
  followPending?: boolean
  avatarBust?: number
  onToggleFollow: () => void
  onEdit: () => void
  onOpenList: (kind: FollowListKind) => void
}

export function ProfileHeader({
  profile,
  followPending,
  avatarBust,
  onToggleFollow,
  onEdit,
  onOpenList,
}: Props): ReactElement {
  const src = useAvatarImage(profile.has_avatar, avatarBust, profile.is_self ? 'me' : profile.id)
  const { followers, following_count: followingCount, following } = profile.follow

  return (
    <Group align="flex-start" justify="space-between" wrap="nowrap" gap="lg">
      <Group align="flex-start" gap="lg" wrap="nowrap">
        <Avatar src={src} size={80} radius="xl" alt="" name={profile.display_name} />
        <Stack gap={4}>
          <Title order={2}>{profile.display_name}</Title>
          {profile.email ? (
            <Text size="sm" c="dimmed">
              {profile.email}
            </Text>
          ) : null}
          {profile.bio ? <Text size="sm">{profile.bio}</Text> : null}
          <Group gap="lg">
            <UnstyledButton onClick={() => onOpenList('followers')}>
              <Text size="sm">{`${followers} follower${followers === 1 ? '' : 's'}`}</Text>
            </UnstyledButton>
            <UnstyledButton onClick={() => onOpenList('following')}>
              <Text size="sm">{`${followingCount} following`}</Text>
            </UnstyledButton>
          </Group>
        </Stack>
      </Group>
      {profile.is_self ? (
        <Button variant="default" onClick={onEdit}>
          Edit profile
        </Button>
      ) : (
        <Button variant={following ? 'default' : 'filled'} loading={followPending} onClick={onToggleFollow}>
          {following ? 'Unfollow' : 'Follow'}
        </Button>
      )}
    </Group>
  )
}
```

Create `apps/web/src/components/profile/ProfileBookmarksList.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Anchor, Card, Group, Image, Loader, Pagination, Pill, SimpleGrid, Stack, Text } from '@mantine/core'
import { PROFILE_PAGE_SIZE, useProfileBookmarks } from '@/hooks'
import { QueryErrorAlert } from '@/components/query-error-alert'

type Props = {
  userId: string
  emptyLabel: string
}

export function ProfileBookmarksList({ userId, emptyLabel }: Props): ReactElement {
  const [page, setPage] = useState(1)
  const bookmarks = useProfileBookmarks(userId, page)

  if (bookmarks.isLoading) return <Loader size="sm" />
  if (bookmarks.isError) {
    return <QueryErrorAlert error={bookmarks.error} onRetry={() => void bookmarks.refetch()} />
  }

  const items = bookmarks.data?.bookmarks ?? []
  const total = bookmarks.data?.total ?? 0
  const pages = Math.ceil(total / PROFILE_PAGE_SIZE)

  if (items.length === 0) return <Text c="dimmed">{emptyLabel}</Text>

  return (
    <Stack gap="md">
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="md">
        {items.map((b) => (
          <Card key={b.id} withBorder padding="md" radius="md">
            <Stack gap="xs">
              <Group gap="xs" wrap="nowrap">
                {b.favicon_url ? <Image src={b.favicon_url} w={16} h={16} alt="" /> : null}
                <Anchor href={b.url} target="_blank" rel="noreferrer" lineClamp={1} fw={600}>
                  {b.title || b.url}
                </Anchor>
              </Group>
              {b.description ? (
                <Text size="sm" c="dimmed" lineClamp={2}>
                  {b.description}
                </Text>
              ) : null}
              {b.tags.length > 0 ? (
                <Group gap={4}>
                  {b.tags.map((tag) => (
                    <Pill key={tag} size="xs">
                      {tag}
                    </Pill>
                  ))}
                </Group>
              ) : null}
            </Stack>
          </Card>
        ))}
      </SimpleGrid>
      {pages > 1 ? <Pagination total={pages} value={page} onChange={setPage} aria-label="Bookmark pages" /> : null}
    </Stack>
  )
}
```

Create `apps/web/src/components/profile/FollowListModal.tsx`:

```tsx
import type { ReactElement } from 'react'
import { Anchor, Loader, Modal, Stack, Text } from '@mantine/core'
import { Link } from '@tanstack/react-router'
import { useProfileFollows } from '@/hooks'
import { QueryErrorAlert } from '@/components/query-error-alert'
import type { FollowListKind } from './ProfileHeader'

type Props = {
  userId: string
  kind: FollowListKind | null
  onClose: () => void
}

export function FollowListModal({ userId, kind, onClose }: Props): ReactElement {
  const list = useProfileFollows(userId, kind ?? 'followers', kind !== null)
  const users = list.data?.users ?? []

  return (
    <Modal opened={kind !== null} onClose={onClose} title={kind === 'following' ? 'Following' : 'Followers'}>
      {list.isLoading ? <Loader size="sm" /> : null}
      {list.isError ? <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} /> : null}
      {list.isSuccess && users.length === 0 ? <Text c="dimmed">Nobody to show.</Text> : null}
      <Stack gap="xs">
        {users.map((u) => (
          <Anchor key={u.id} component={Link} to="/u/$id" params={{ id: u.id }} onClick={onClose}>
            {u.display_name}
          </Anchor>
        ))}
      </Stack>
    </Modal>
  )
}
```

Create `apps/web/src/components/profile/index.ts`:

```ts
export { FollowListModal } from './FollowListModal'
export { ProfileBookmarksList } from './ProfileBookmarksList'
export { ProfileHeader, type FollowListKind } from './ProfileHeader'
```

Add `export * from './profile'` to `components/index.ts`.

- [ ] **Step 4: Implement the page**

Create `apps/web/src/pages/user-profile/UserProfilePage.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Center, Loader, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useNavigate, useParams } from '@tanstack/react-router'
import { FollowListModal, ProfileBookmarksList, ProfileHeader, type FollowListKind } from '@/components'
import { useFollowUser, useProfile, useUnfollowUser } from '@/hooks'

export function UserProfileView({ userId, onEdit }: { userId: string; onEdit: () => void }): ReactElement {
  const profile = useProfile(userId)
  const follow = useFollowUser()
  const unfollow = useUnfollowUser()
  const [list, setList] = useState<FollowListKind | null>(null)

  if (profile.isLoading) {
    return (
      <Center py="xl">
        <Loader size="sm" />
      </Center>
    )
  }
  if (!profile.data) {
    return (
      <Stack gap="xs">
        <Title order={3}>Profile not available</Title>
        <Text c="dimmed">This link may be out of date.</Text>
      </Stack>
    )
  }

  const data = profile.data
  const pending = follow.isPending || unfollow.isPending

  async function toggleFollow(): Promise<void> {
    try {
      await (data.follow.following ? unfollow : follow).mutateAsync(data.id)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Stack gap="xl">
      <ProfileHeader
        profile={data}
        followPending={pending}
        onToggleFollow={() => void toggleFollow()}
        onEdit={onEdit}
        onOpenList={setList}
      />
      <ProfileBookmarksList userId={data.id} emptyLabel="Nothing to show yet." />
      <FollowListModal userId={data.id} kind={list} onClose={() => setList(null)} />
    </Stack>
  )
}

export function UserProfilePage(): ReactElement {
  const { id } = useParams({ from: '/_app/u/$id' })
  const navigate = useNavigate()
  return <UserProfileView userId={id} onEdit={() => void navigate({ to: '/profile' })} />
}
```

A server failure also lands on "Profile not available", which is deliberate: the page must not distinguish a denial from anything else.

Create `apps/web/src/pages/user-profile/index.ts` with `export { UserProfilePage } from './UserProfilePage'`, and add `export * from './user-profile'` to `pages/index.ts`.

Create `apps/web/src/routes/_app/u.$id.tsx`:

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { UserProfilePage } from '@/pages'

export const Route = createFileRoute('/_app/u/$id')({
  component: UserProfilePage,
})
```

- [ ] **Step 5: Generate routes and run the tests**

Run: `cd apps/web && pnpm routes:gen && pnpm exec vitest run src/components/profile src/pages/user-profile && pnpm exec tsc -b 2>&1 | grep -c "error TS"`
Expected: tests pass, `0` type errors.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src
git commit -m "Add the user profile page."
```

---

### Task 9: Own profile revamp

**Files:**
- Create: `apps/web/src/pages/profile/ProfileEditDrawer.tsx`
- Modify: `apps/web/src/pages/profile/ProfilePage.tsx`
- Modify: `apps/web/src/pages/profile/ProfileAboutCard.tsx` if its layout needs the card removed
- Delete: `apps/web/src/pages/profile/ProfileFollowsCard.tsx`

- [ ] **Step 1: Create the drawer**

Create `apps/web/src/pages/profile/ProfileEditDrawer.tsx`:

```tsx
import type { ReactElement, ReactNode } from 'react'
import { Button, Drawer, Group, Select, Stack, Text } from '@mantine/core'
import type { ProfileVisibility } from '@/schemas'

const OPTIONS = [
  { value: 'hidden', label: 'Hidden' },
  { value: 'followers', label: 'Followers' },
  { value: 'members', label: 'Members' },
]

type Props = {
  opened: boolean
  visibility: ProfileVisibility
  saving: boolean
  onVisibility: (next: ProfileVisibility) => void
  onSaveVisibility: () => void
  onClose: () => void
  children: ReactNode
}

export function ProfileEditDrawer({
  opened,
  visibility,
  saving,
  onVisibility,
  onSaveVisibility,
  onClose,
  children,
}: Props): ReactElement {
  return (
    <Drawer opened={opened} onClose={onClose} position="right" size="lg" title="Edit profile">
      <Stack gap="xl">
        {children}
        <Stack gap="xs">
          <Select
            label="Who can see my profile"
            data={OPTIONS}
            value={visibility}
            allowDeselect={false}
            onChange={(next) => next && onVisibility(next as ProfileVisibility)}
          />
          <Text size="xs" c="dimmed">
            Members means anyone signed in here. Use a Public link for people without an account.
          </Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={onClose}>
              Cancel
            </Button>
            <Button loading={saving} onClick={onSaveVisibility}>
              Save
            </Button>
          </Group>
        </Stack>
      </Stack>
    </Drawer>
  )
}
```

- [ ] **Step 2: Rebuild `ProfilePage`**

In `apps/web/src/pages/profile/ProfilePage.tsx`:

Replace `ProfileFollowsCard`'s import with `import { ProfileEditDrawer } from './ProfileEditDrawer'`. Add `FollowListModal`, `ProfileBookmarksList`, `ProfileHeader` and `type FollowListKind` to the `@/components` import, and `useProfile` to the `@/hooks` import. Add `import type { ProfileVisibility } from '@/schemas'`.

Add state beside the others:

```tsx
  const [editOpen, setEditOpen] = useState(false)
  const [list, setList] = useState<FollowListKind | null>(null)
  const [visibility, setVisibility] = useState<ProfileVisibility>('hidden')
  const profile = useProfile(user?.id ?? '')
```

`useProfile` must not fire before `user` loads. Add an `enabled` parameter in `hooks/profiles/useProfiles.ts`: `useProfile(userId: string, enabled = true)` with `enabled: enabled && !!userId` in the query, and call `useProfile(user?.id ?? '', !!user)`.

Add to the `useSyncDuringRender([user], ...)` body:

```tsx
    setVisibility(user.profile_visibility)
```

Replace the `<>...</>` branch so the page reads:

```tsx
        <>
          {profile.data ? (
            <ProfileHeader
              profile={profile.data}
              avatarBust={avatarBust}
              onToggleFollow={() => undefined}
              onEdit={() => setEditOpen(true)}
              onOpenList={setList}
            />
          ) : null}

          {visibility === 'hidden' ? (
            <Text size="sm" c="dimmed">
              Your profile is hidden. Other people can't open it, including followers.
            </Text>
          ) : null}

          <Stack gap="xs">
            <Title order={4}>What others can see</Title>
            <ProfileBookmarksList
              userId={user.id}
              emptyLabel="Nothing yet. Set a category to Followers or Members to show its bookmarks here."
            />
          </Stack>

          <ProfileBlocksCard />

          <FollowListModal userId={user.id} kind={list} onClose={() => setList(null)} />

          <ProfileEditDrawer
            opened={editOpen}
            visibility={visibility}
            saving={saveProfile.isPending}
            onVisibility={setVisibility}
            onSaveVisibility={() => {
              saveProfile.mutate(
                { profile_visibility: visibility },
                {
                  onSuccess: () => notifications.show({ message: 'Profile saved' }),
                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                }
              )
            }}
            onClose={() => setEditOpen(false)}
          >
            {/* existing <ProfileAboutCard ... /> and <ProfilePasswordCard ... /> move here unchanged */}
          </ProfileEditDrawer>
        </>
```

Move the two existing elements into the drawer where the comment sits, with their props unchanged, and delete the comment. Remove `maw={560}` from the page's outer `Stack` so the bookmark grid has room, and keep it on the two cards' wrapper only if they look too wide in the drawer. Add `Text` to the Mantine import.

Delete `ProfileFollowsCard.tsx`. Its only user is this page.

- [ ] **Step 3: Check it compiles and tests pass**

Run: `cd apps/web && pnpm exec tsc -b 2>&1 | grep -c "error TS"; pnpm test 2>&1 | grep -E "Test Files|Tests  |FAIL"`
Expected: `0`, all pass.

- [ ] **Step 4: Commit**

```bash
git add -A apps/web/src
git commit -m "Rebuild the profile page around the header and an edit drawer."
```

---

### Task 10: Visibility controls and the profile link

**Files:**
- Create: `apps/web/src/components/category/CategoryVisibilitySelect.tsx`, `CategoryVisibilitySelect.test.tsx`
- Modify: `apps/web/src/components/category/EditCategoryDrawer.tsx`
- Modify: `apps/web/src/components/bookmark/BookmarkDrawer.tsx`
- Modify: `apps/web/src/services/bookmarks/bookmarks.ts`, `bookmarks.test.ts`
- Modify: `apps/web/src/components/bookmark/BookmarkCardMenu.tsx`, `BookmarkCard.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/components/category/CategoryVisibilitySelect.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CategoryVisibilitySelect } from './CategoryVisibilitySelect'

function renderSelect(isVault: boolean): void {
  render(
    <MantineProvider>
      <CategoryVisibilitySelect value="private" isVault={isVault} onChange={vi.fn()} />
    </MantineProvider>
  )
}

describe('CategoryVisibilitySelect', () => {
  afterEach(cleanup)

  it('is disabled for vault categories', () => {
    renderSelect(true)
    expect((screen.getByLabelText('Who can see this') as HTMLInputElement).disabled).toBe(true)
  })

  it('explains that Members is not a Public link', () => {
    renderSelect(false)
    expect(screen.getByText(/use a public link for people without an account/i)).toBeTruthy()
  })
})
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd apps/web && pnpm exec vitest run src/components/category/CategoryVisibilitySelect`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement the select**

Create `apps/web/src/components/category/CategoryVisibilitySelect.tsx`:

```tsx
import type { ReactElement } from 'react'
import { Select } from '@mantine/core'
import type { CategoryVisibility } from '@/schemas'

const OPTIONS = [
  { value: 'private', label: 'Only me' },
  { value: 'followers', label: 'Followers' },
  { value: 'members', label: 'Members' },
]

type Props = {
  value: CategoryVisibility
  isVault: boolean
  onChange: (next: CategoryVisibility) => void
}

export function CategoryVisibilitySelect({ value, isVault, onChange }: Props): ReactElement {
  return (
    <Select
      label="Who can see this"
      description="Members means anyone signed in here. Use a Public link for people without an account."
      data={OPTIONS}
      value={value}
      allowDeselect={false}
      disabled={isVault}
      onChange={(next) => next && onChange(next as CategoryVisibility)}
    />
  )
}
```

- [ ] **Step 4: Wire `EditCategoryDrawer`**

In `components/category/EditCategoryDrawer.tsx`:

Add the imports `import { CategoryVisibilitySelect } from './CategoryVisibilitySelect'` and `import type { CategoryVisibility } from '@/schemas'`.

Add state: `const [visibility, setVisibility] = useState<CategoryVisibility>('private')`.

Add to the `useSyncDuringRender` body: `setVisibility(category.visibility)`.

In `onSave`, beside `hiddenChanged`:

```tsx
    const visibilityChanged = isOwner && visibility !== category.visibility
```

Add `&& !visibilityChanged` to the no-change early return, and to the update body:

```tsx
          ...(visibilityChanged ? { visibility } : {}),
```

Render inside the existing `{isOwner ? (...) : null}` area, directly after the Hidden checkbox block:

```tsx
          {isOwner ? (
            <CategoryVisibilitySelect
              value={visibility}
              isVault={!!category?.is_vault}
              onChange={setVisibility}
            />
          ) : null}
```

- [ ] **Step 5: Add `is_private` to the bookmark draft**

Add to `BookmarkDraft` in `services/bookmarks/bookmarks.ts`: `is_private: boolean`. Add `is_private: false,` to `emptyDraft`, and `is_private: !!b.is_private,` to `draftFromBookmark`. Add `is_private: false` to the expected object in `services/bookmarks/bookmarks.test.ts`.

In `components/bookmark/BookmarkDrawer.tsx`, add `Switch` to the Mantine import. In the update body in `save`, after `is_pinned: draft.is_pinned,`:

```tsx
            ...(isOwner && !draft.is_vault ? { is_private: draft.is_private } : {}),
```

Render above the preview field:

```tsx
          {isOwner && !draft.is_vault ? (
            <Switch
              label="Only me"
              description="Keep this off your profile, even in a visible category."
              checked={draft.is_private}
              onChange={(e) => {
                const is_private = e.currentTarget.checked
                setDraft((d) => ({ ...d, is_private }))
              }}
            />
          ) : null}
```

A write invitee must not send `is_private`, because the API answers 403. Add `owner_user_id?: string` to `BookmarkDraft`, and `owner_user_id: b.owner_user_id,` to `draftFromBookmark` and to the expected object in `bookmarks.test.ts`. In `BookmarkDrawer`, add `useMe` to the `@/hooks` import and compute:

```tsx
  const me = useMe(true)
  const isOwner = !editing || draft.owner_user_id === me.data?.user.id
```

- [ ] **Step 6: Add "View profile"**

In `components/bookmark/BookmarkCardMenu.tsx`, add a prop `onViewOwnerProfile?: () => void`, and directly above the `Menu.Item` that calls `onToggleFollowOwner`:

```tsx
          {onViewOwnerProfile ? <Menu.Item onClick={onViewOwnerProfile}>View profile</Menu.Item> : null}
```

In `components/bookmark/BookmarkCard.tsx`, change the router import to `import { Link, useNavigate } from '@tanstack/react-router'`, add `const navigate = useNavigate()` beside `owner`, and pass this next to `onToggleFollowOwner` at line 153, under the same condition that passes `onToggleFollowOwner`:

```tsx
              onViewOwnerProfile={() => void navigate({ to: '/u/$id', params: { id: b.owner_user_id! } })}
```

`BookmarkCard` also renders on the public list at `/l/$slug` with no signed-in user. Pass the prop only where `onToggleFollowOwner` is passed, and confirm in the browser that the item does not appear on a Public link page.

A profile that returns 404 lands on "Profile not available", which is expected for a hidden owner.

- [ ] **Step 7: Run the checks**

Run: `cd apps/web && pnpm exec tsc -b 2>&1 | grep -c "error TS"; pnpm test 2>&1 | grep -E "Test Files|Tests  |FAIL"; pnpm lint 2>&1 | grep -cE "warning|error"; pnpm exec prettier --check src | tail -1`
Expected: `0`, all pass, `0`, Prettier clean.

- [ ] **Step 8: Commit**

```bash
git add -A apps/web/src
git commit -m "Add visibility controls and a View profile action."
```

---

### Task 11: Smoke test, docs and release checks

**Files:**
- Modify: `docs/ROADMAP.md`, `docs/FEATURES.md`

- [ ] **Step 1: Smoke-test the routes**

Start the throwaway container from Conventions, then:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
docker exec linkdean-pg-check psql -U linkdean -c 'create database smoke'
cargo build -p linkdean-api
```

Read how the previous smoke run set its environment (`docker compose` values in `deploy/docker/docker-compose.yml` and `crates/api/src/state/mod.rs` for required variables), start `./target/debug/linkdean-api` on port 8081 with `DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:55433/smoke`, and poll `/api/health`. The first registration needs the bootstrap path, so register a throwaway admin first.

Register users A and B, then assert with curl using `Authorization: Bearer <token>`:

| Request | Expected |
| --- | --- |
| B: `GET /api/users/{A}/profile` before A opts in | 404 |
| No token: `GET /api/users/{A}/profile` | 401 |
| A: `PATCH /api/users/me` `{"profile_visibility":"members"}` | 200 |
| B: `GET /api/users/{A}/profile` | 200, body has no `email` key |
| A: `GET /api/users/{A}/profile` | 200, body has `email` |
| B: `GET /api/users/{A}/bookmarks?limit=500` | 400 |
| B: `GET /api/users/{A}/followers` | 200, no `email` anywhere in the body |
| B: `GET /api/users/{A}/avatar` | 404 until A uploads one |

Kill the API process and remove the container with `docker rm -f linkdean-pg-check`.

- [ ] **Step 2: Check it in the browser**

Run the app and confirm these by hand: a second user opens `/u/{id}` after the owner opts in; a Followers category appears only after following; "Only me" removes a bookmark; a vault category's select is disabled; the "View profile" item is absent on a Public link page; the bookmark, category, profile, register, duplicate-merge and avatar-crop forms still populate and reset. The last check also covers the effect refactor in `d070085`, which has only had type and unit checks so far.

- [ ] **Step 3: Update the docs**

Remove from `docs/ROADMAP.md` the whole "Profile (own & others) + library visibility" item and its nested list. Add to `docs/FEATURES.md`, after the block and unblock line:

```markdown
- Profiles (`/u/{id}`): avatar, name, bio, follower and following lists, follow button, and bookmarks the owner chose to show. Own profile adds email and an edit drawer.
- Visibility: profile is Hidden, Followers or Members; categories are Only me, Followers or Members; a bookmark can be Only me. Everything starts private. Members means anyone signed in, and Public links stay the only access without an account.
```

- [ ] **Step 4: Run everything**

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo fmt --all --check
cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -cE "^crates.*(warning|error)"
DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:55433/linkdean cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED"
cd apps/web && pnpm exec tsc -b && pnpm lint && pnpm test && pnpm build
graphify update .
```

Expected: clippy `0`, no failed tests, a clean lint, and a successful build. The `cargo test` line needs the container running, so start it again if Step 1 removed it.

- [ ] **Step 5: Commit**

```bash
git add docs
git commit -m "Record profiles and visibility as shipped."
```

Push only when Arsam says so.
