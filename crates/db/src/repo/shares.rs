use linkdean_core::{
    Category, CategoryAccess, CategoryShare, CategoryShareView, CreateCategoryShare, Error, Result,
    ShareAccess, UpdateCategoryShare,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::categories::get_category_by_id;
use super::users::find_user_by_email;

#[derive(Debug, sqlx::FromRow)]
struct ShareRow {
    id: Uuid,
    category_id: Uuid,
    invitee_user_id: Uuid,
    access: String,
    invited_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    invitee_email: String,
    invitee_display_name: Option<String>,
}

fn to_view(row: ShareRow) -> Result<CategoryShareView> {
    let access = ShareAccess::parse(&row.access).map_err(Error::BadRequest)?;
    Ok(CategoryShareView {
        id: row.id,
        category_id: row.category_id,
        invitee_user_id: row.invitee_user_id,
        invitee_email: row.invitee_email,
        invitee_display_name: row.invitee_display_name,
        access,
        invited_by_user_id: row.invited_by_user_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// Resolve effective access for `user_id` on `category_id`.
/// Owner wins. Else nearest ancestor share (walking parent chain). Vault → NotFound for non-owners.
pub async fn category_access(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Uuid,
) -> Result<CategoryAccess> {
    let cat = get_category_by_id(pool, category_id).await?;
    if cat.owner_user_id == user_id {
        return Ok(CategoryAccess::Owner);
    }
    if cat.is_vault {
        return Err(Error::NotFound);
    }

    let mut current = Some(cat);
    while let Some(c) = current {
        if let Some(share) = find_direct_share(pool, c.id, user_id).await? {
            let access = ShareAccess::parse(&share.access).map_err(Error::BadRequest)?;
            return Ok(access.to_category_access());
        }
        current = match c.parent_id {
            Some(pid) => Some(get_category_by_id(pool, pid).await?),
            None => None,
        };
    }
    Err(Error::NotFound)
}

pub async fn require_category_read(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Uuid,
) -> Result<(Category, CategoryAccess)> {
    let access = category_access(pool, user_id, category_id).await?;
    let cat = get_category_by_id(pool, category_id).await?;
    Ok((cat, access))
}

pub async fn require_category_write(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Uuid,
) -> Result<(Category, CategoryAccess)> {
    let (cat, access) = require_category_read(pool, user_id, category_id).await?;
    if !access.can_write() {
        return Err(Error::Forbidden);
    }
    Ok((cat, access))
}

pub async fn require_category_owner(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Uuid,
) -> Result<Category> {
    let (cat, access) = require_category_read(pool, user_id, category_id).await?;
    if !access.is_owner() {
        return Err(Error::Forbidden);
    }
    Ok(cat)
}

async fn find_direct_share(
    pool: &PgPool,
    category_id: Uuid,
    invitee_user_id: Uuid,
) -> Result<Option<CategoryShare>> {
    sqlx::query_as::<_, CategoryShare>(
        r#"
        SELECT id, category_id, invitee_user_id, access, invited_by_user_id, created_at, updated_at
        FROM category_shares
        WHERE category_id = $1 AND invitee_user_id = $2
        "#,
    )
    .bind(category_id)
    .bind(invitee_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

/// Categories visible via shares (shared root + non-vault descendants), with grant access.
pub async fn list_shared_categories(
    pool: &PgPool,
    invitee_user_id: Uuid,
    include_hidden: bool,
) -> Result<Vec<(Category, CategoryAccess)>> {
    let rows = sqlx::query_as::<_, CategoryShareRow>(
        r#"
        WITH RECURSIVE tree AS (
            SELECT
                c.id, c.owner_user_id, c.parent_id, c.name, c.is_hidden, c.is_vault,
                c.icon, c.position, c.deleted_at, c.created_at, c.updated_at,
                cs.access AS share_access
            FROM category_shares cs
            JOIN categories c ON c.id = cs.category_id
            WHERE cs.invitee_user_id = $1
              AND c.is_vault = FALSE
              AND c.deleted_at IS NULL
              AND ($2 OR c.is_hidden = FALSE)
            UNION ALL
            SELECT
                child.id, child.owner_user_id, child.parent_id, child.name, child.is_hidden,
                child.is_vault, child.icon, child.position, child.deleted_at, child.created_at,
                child.updated_at, tree.share_access
            FROM categories child
            JOIN tree ON child.parent_id = tree.id
            WHERE child.is_vault = FALSE
              AND child.deleted_at IS NULL
              AND ($2 OR child.is_hidden = FALSE)
        )
        SELECT DISTINCT ON (id)
            id, owner_user_id, parent_id, name, is_hidden, is_vault, icon, position,
            deleted_at, created_at, updated_at, share_access
        FROM tree
        ORDER BY id, CASE share_access WHEN 'write' THEN 0 ELSE 1 END
        "#,
    )
    .bind(invitee_user_id)
    .bind(include_hidden)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let access = ShareAccess::parse(&row.share_access)
            .map_err(Error::BadRequest)?
            .to_category_access();
        out.push((row.into_category(), access));
    }
    Ok(out)
}

#[derive(Debug, sqlx::FromRow)]
struct CategoryShareRow {
    id: Uuid,
    owner_user_id: Uuid,
    parent_id: Option<Uuid>,
    name: String,
    is_hidden: bool,
    is_vault: bool,
    icon: Option<String>,
    position: i32,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    share_access: String,
}

impl CategoryShareRow {
    fn into_category(self) -> Category {
        Category {
            id: self.id,
            owner_user_id: self.owner_user_id,
            parent_id: self.parent_id,
            name: self.name,
            is_hidden: self.is_hidden,
            is_vault: self.is_vault,
            vault_id: None,
            icon: self.icon,
            // Public slug is owner-private; shared listing never exposes it.
            public_slug: None,
            // Visibility is an owner setting; the shared listing does not carry it.
            visibility: "private".to_string(),
            position: self.position,
            deleted_at: self.deleted_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

pub async fn list_shares_for_category(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
) -> Result<Vec<CategoryShareView>> {
    require_category_owner(pool, owner_user_id, category_id).await?;
    let rows = sqlx::query_as::<_, ShareRow>(
        r#"
        SELECT
            cs.id, cs.category_id, cs.invitee_user_id, cs.access, cs.invited_by_user_id,
            cs.created_at, cs.updated_at,
            u.email AS invitee_email, u.display_name AS invitee_display_name
        FROM category_shares cs
        JOIN users u ON u.id = cs.invitee_user_id
        WHERE cs.category_id = $1
        ORDER BY u.email
        "#,
    )
    .bind(category_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter().map(to_view).collect()
}

pub async fn create_share(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
    req: CreateCategoryShare,
) -> Result<linkdean_core::CreateCategoryShareOutcome> {
    use linkdean_core::{CreateCategoryShareOutcome, PendingShareSubjectType};

    let cat = require_category_owner(pool, owner_user_id, category_id).await?;
    if cat.is_vault {
        return Err(Error::BadRequest("cannot share vault categories".into()));
    }

    let email = req.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("valid email required".into()));
    }

    let Some(invitee) = find_user_by_email(pool, &email).await? else {
        let invite = super::pending_shares::upsert_pending_share_invite(
            pool,
            owner_user_id,
            PendingShareSubjectType::Category,
            category_id,
            &email,
            req.access,
            None,
        )
        .await?
        .0;
        return Ok(CreateCategoryShareOutcome::Pending { invite });
    };
    if invitee.id == owner_user_id {
        return Err(Error::BadRequest("cannot share with yourself".into()));
    }
    super::blocks::assert_not_blocked(pool, owner_user_id, invitee.id).await?;

    let existed = find_direct_share(pool, category_id, invitee.id)
        .await?
        .is_some();
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
                pool,
                owner_user_id,
                invitee.id,
                category_id,
                &cat.name,
                invite.access,
            )
            .await;
        }
        return Ok(CreateCategoryShareOutcome::Pending { invite });
    }
    let access = req.access.as_str();
    let row = sqlx::query_as::<_, ShareRow>(
        r#"
        INSERT INTO category_shares (category_id, invitee_user_id, access, invited_by_user_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (category_id, invitee_user_id) DO UPDATE
          SET access = EXCLUDED.access, updated_at = now()
        RETURNING
            id, category_id, invitee_user_id, access, invited_by_user_id, created_at, updated_at,
            (SELECT email FROM users WHERE id = invitee_user_id) AS invitee_email,
            (SELECT display_name FROM users WHERE id = invitee_user_id) AS invitee_display_name
        "#,
    )
    .bind(category_id)
    .bind(invitee.id)
    .bind(access)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let view = to_view(row)?;
    super::notifications::notify_category_share_access(
        pool,
        owner_user_id,
        invitee.id,
        category_id,
        &cat.name,
        view.access,
    )
    .await;
    Ok(CreateCategoryShareOutcome::Active { share: view })
}

pub async fn update_share(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
    invitee_user_id: Uuid,
    req: UpdateCategoryShare,
) -> Result<CategoryShareView> {
    let cat = require_category_owner(pool, owner_user_id, category_id).await?;
    let access = req.access.as_str();
    let row = sqlx::query_as::<_, ShareRow>(
        r#"
        UPDATE category_shares
        SET access = $3, updated_at = now()
        WHERE category_id = $1 AND invitee_user_id = $2
        RETURNING
            id, category_id, invitee_user_id, access, invited_by_user_id, created_at, updated_at,
            (SELECT email FROM users WHERE id = invitee_user_id) AS invitee_email,
            (SELECT display_name FROM users WHERE id = invitee_user_id) AS invitee_display_name
        "#,
    )
    .bind(category_id)
    .bind(invitee_user_id)
    .bind(access)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    let view = to_view(row)?;
    super::notifications::notify_category_share_access(
        pool,
        owner_user_id,
        invitee_user_id,
        category_id,
        &cat.name,
        view.access,
    )
    .await;
    Ok(view)
}

pub async fn delete_share(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
    invitee_user_id: Uuid,
) -> Result<()> {
    let cat = require_category_owner(pool, owner_user_id, category_id).await?;
    let res =
        sqlx::query("DELETE FROM category_shares WHERE category_id = $1 AND invitee_user_id = $2")
            .bind(category_id)
            .bind(invitee_user_id)
            .execute(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    super::notifications::notify_category_share_revoked(
        pool,
        owner_user_id,
        invitee_user_id,
        category_id,
        &cat.name,
    )
    .await;
    Ok(())
}

/// True if user can read the bookmark via ownership or a linked category share.
pub async fn bookmark_can_read(pool: &PgPool, user_id: Uuid, bookmark_id: Uuid) -> Result<bool> {
    let owned: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM bookmarks WHERE id = $1 AND owner_user_id = $2")
            .bind(bookmark_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    if owned.is_some() {
        return Ok(true);
    }

    let cat_ids: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT category_id FROM bookmark_categories WHERE bookmark_id = $1
        UNION
        SELECT category_id FROM bookmarks WHERE id = $1 AND category_id IS NOT NULL
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    for (cid,) in cat_ids {
        if category_access(pool, user_id, cid).await.is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// True if user can mutate bookmark: owns it, or has write on any linked category.
pub async fn bookmark_can_write(pool: &PgPool, user_id: Uuid, bookmark_id: Uuid) -> Result<bool> {
    let owned: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM bookmarks WHERE id = $1 AND owner_user_id = $2")
            .bind(bookmark_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    if owned.is_some() {
        return Ok(true);
    }

    let cat_ids: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT category_id FROM bookmark_categories WHERE bookmark_id = $1
        UNION
        SELECT category_id FROM bookmarks WHERE id = $1 AND category_id IS NOT NULL
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    for (cid,) in cat_ids {
        match category_access(pool, user_id, cid).await {
            Ok(a) if a.can_write() => return Ok(true),
            Ok(_) => {}
            Err(Error::NotFound) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(false)
}

/// Category ids the owner has shared out (share roots + their non-vault descendants).
pub async fn owned_shared_out_category_ids(
    pool: &PgPool,
    owner_user_id: Uuid,
) -> Result<std::collections::HashSet<Uuid>> {
    Ok(owned_shared_out_invitees(pool, owner_user_id)
        .await?
        .into_keys()
        .collect())
}

/// Per owned category: invitee labels (display name / email local-part) for share tooltip.
/// Includes share roots and their non-vault descendants.
pub async fn owned_shared_out_invitees(
    pool: &PgPool,
    owner_user_id: Uuid,
) -> Result<std::collections::HashMap<Uuid, Vec<String>>> {
    let rows: Vec<(Uuid, Option<String>, String)> = sqlx::query_as(
        r#"
        WITH RECURSIVE tree AS (
            SELECT
                c.id,
                u.display_name AS invitee_display_name,
                u.email AS invitee_email
            FROM category_shares cs
            JOIN categories c ON c.id = cs.category_id
            JOIN users u ON u.id = cs.invitee_user_id
            WHERE c.owner_user_id = $1
              AND c.is_vault = FALSE
            UNION
            SELECT
                child.id,
                tree.invitee_display_name,
                tree.invitee_email
            FROM categories child
            JOIN tree ON child.parent_id = tree.id
            WHERE child.owner_user_id = $1
              AND child.is_vault = FALSE
        )
        SELECT id, invitee_display_name, invitee_email FROM tree
        "#,
    )
    .bind(owner_user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut map: std::collections::HashMap<Uuid, Vec<String>> = std::collections::HashMap::new();
    for (id, display_name, email) in rows {
        let label = super::users::share_invitee_label(display_name.as_deref(), &email, "User");
        let entry = map.entry(id).or_default();
        if !entry.iter().any(|s| s == &label) {
            entry.push(label);
        }
    }
    for labels in map.values_mut() {
        labels.sort();
    }
    Ok(map)
}

/// Invitee labels when any of the bookmark's folders (or an ancestor) is shared out.
pub async fn bookmark_shared_with(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(Option<String>, String)> = sqlx::query_as(
        r#"
        WITH RECURSIVE bm_cats AS (
            SELECT category_id AS id
            FROM bookmark_categories
            WHERE bookmark_id = $1
            UNION
            SELECT category_id
            FROM bookmarks
            WHERE id = $1 AND category_id IS NOT NULL
        ),
        ancestors AS (
            SELECT c.id, c.parent_id
            FROM categories c
            JOIN bm_cats b ON c.id = b.id
            UNION ALL
            SELECT c.id, c.parent_id
            FROM categories c
            JOIN ancestors a ON a.parent_id = c.id
        )
        SELECT DISTINCT u.display_name, u.email
        FROM category_shares cs
        JOIN users u ON u.id = cs.invitee_user_id
        WHERE cs.category_id IN (SELECT id FROM ancestors)
        ORDER BY u.email
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut labels = Vec::new();
    for (display_name, email) in rows {
        let label = super::users::share_invitee_label(display_name.as_deref(), &email, "User");
        if !labels.iter().any(|s| s == &label) {
            labels.push(label);
        }
    }
    labels.sort();
    Ok(labels)
}

/// True when any of the bookmark's folders (or an ancestor) is shared out by the owner.
pub async fn bookmark_shared_out(pool: &PgPool, bookmark_id: Uuid) -> Result<bool> {
    Ok(!bookmark_shared_with(pool, bookmark_id).await?.is_empty())
}
