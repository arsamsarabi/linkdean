use std::collections::HashSet;

use linkdean_core::crypto;
use linkdean_core::{
    Bookmark, Category, Error, MoveToVaultRequest, Result, VaultKeyring, VaultMoveBlocker,
    VaultMoveBlockerKind, VaultMoveOutcome, VaultPayload,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::bookmarks::bookmark_tag_names;
use super::tags::ensure_tags;
use super::vault::get_user_vault;

const SCRUBBED_TITLE: &str = "Vault item";
const SCRUBBED_SUMMARY: &str = "Moved to a vault";

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Categories with all their subcategories, owned by this user.
async fn expand_categories(pool: &PgPool, user_id: Uuid, ids: &[Uuid]) -> Result<Vec<Category>> {
    let rows = sqlx::query_as::<_, Category>(
        r#"
        WITH RECURSIVE tree AS (
            SELECT * FROM categories WHERE id = ANY($1)
            UNION
            SELECT c.* FROM categories c JOIN tree t ON c.parent_id = t.id
        )
        SELECT * FROM tree
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let found: HashSet<Uuid> = rows.iter().map(|c| c.id).collect();
    if ids.iter().any(|id| !found.contains(id)) || rows.iter().any(|c| c.owner_user_id != user_id) {
        return Err(Error::NotFound);
    }
    Ok(rows)
}

/// Requested bookmarks plus everything in the moved categories, trashed included.
async fn collect_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    requested: &[Uuid],
    category_ids: &[Uuid],
) -> Result<Vec<Bookmark>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT DISTINCT b.* FROM bookmarks b
        LEFT JOIN bookmark_categories bc ON bc.bookmark_id = b.id
        WHERE b.id = ANY($1) OR bc.category_id = ANY($2) OR b.category_id = ANY($2)
        "#,
    )
    .bind(requested)
    .bind(category_ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let found: HashSet<Uuid> = rows.iter().map(|b| b.id).collect();
    if requested.iter().any(|id| !found.contains(id))
        || rows.iter().any(|b| b.owner_user_id != user_id)
    {
        return Err(Error::NotFound);
    }
    Ok(rows)
}

/// A bookmark's readable content, whichever side of a vault it is on.
async fn read_payload(
    pool: &PgPool,
    bookmark: &Bookmark,
    keyring: &VaultKeyring,
) -> Result<VaultPayload> {
    let Some(vault_id) = bookmark.vault_id else {
        return Ok(VaultPayload {
            url: bookmark.url.clone(),
            title: bookmark.title.clone(),
            description: bookmark.description.clone(),
            notes: bookmark.notes.clone(),
            tags: bookmark_tag_names(pool, bookmark.id).await?,
        });
    };
    let key = *keyring.get(vault_id).ok_or(Error::VaultLocked)?;
    let ct = bookmark
        .vault_ciphertext
        .clone()
        .ok_or(Error::msg("vault blob missing"))?;
    let nonce = bookmark
        .vault_nonce
        .clone()
        .ok_or(Error::msg("vault nonce missing"))?;
    super::blocking::run_blocking(move || crypto::decrypt_payload(&key, &ct, &nonce)).await
}

async fn find_blockers(
    pool: &PgPool,
    categories: &[Category],
    bookmarks: &[(Bookmark, VaultPayload)],
    into_vault: bool,
) -> Result<Vec<VaultMoveBlocker>> {
    let moving: Vec<Uuid> = categories.iter().map(|c| c.id).collect();
    let mut out = Vec::new();

    for category in categories.iter().filter(|_| into_vault) {
        let (people,): (i64,) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM category_shares WHERE category_id = $1)
                + (SELECT COUNT(*) FROM pending_share_invites
                   WHERE subject_type = 'category' AND subject_id = $1
                     AND claimed_at IS NULL AND revoked_at IS NULL)
            "#,
        )
        .bind(category.id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
        if people > 0 {
            let noun = if people == 1 { "person" } else { "people" };
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::Shared,
                subject_id: category.id,
                related_id: None,
                message: format!("Remove {people} {noun} from sharing on “{}”", category.name),
            });
        }
        if category.public_slug.is_some() {
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::PublicLink,
                subject_id: category.id,
                related_id: None,
                message: format!("Turn off the public link on “{}”", category.name),
            });
        }
    }

    for (bookmark, payload) in bookmarks {
        let others: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT c.id, c.name FROM bookmark_categories bc
            JOIN categories c ON c.id = bc.category_id
            WHERE bc.bookmark_id = $1 AND NOT (c.id = ANY($2))
            ORDER BY c.name
            "#,
        )
        .bind(bookmark.id)
        .bind(&moving)
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
        for (category_id, name) in others {
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::OtherCategory,
                subject_id: bookmark.id,
                related_id: Some(category_id),
                message: format!("Remove “{}” from “{name}”", payload.title),
            });
        }

        if !into_vault {
            continue;
        }

        let boards: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT DISTINCT d.id, d.title FROM dashboard_items i
            JOIN dashboard_sections s ON s.id = i.section_id
            JOIN dashboards d ON d.id = s.dashboard_id
            WHERE i.bookmark_id = $1 AND d.deleted_at IS NULL
            "#,
        )
        .bind(bookmark.id)
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
        for (dashboard_id, title) in boards {
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::Dashboard,
                subject_id: bookmark.id,
                related_id: Some(dashboard_id),
                message: format!("Remove “{}” from dashboard “{title}”", payload.title),
            });
        }
    }
    Ok(out)
}

async fn write_encrypted(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    vault_id: Uuid,
    ct: Vec<u8>,
    nonce: Vec<u8>,
) -> Result<Vec<Uuid>> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET is_vault = TRUE, vault_id = $1, vault_ciphertext = $2, vault_nonce = $3,
            url = '', title = '', description = '', notes = NULL,
            favicon_url = NULL, preview_image_url = NULL, has_preview = FALSE,
            is_private = FALSE,
            link_check_status = NULL, link_check_status_code = NULL,
            link_check_error = NULL, link_checked_at = NULL, link_check_fail_count = 0,
            updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(vault_id)
    .bind(ct)
    .bind(nonce)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    let tag_ids: Vec<Uuid> =
        sqlx::query_scalar("DELETE FROM bookmark_tags WHERE bookmark_id = $1 RETURNING tag_id")
            .bind(id)
            .fetch_all(&mut **tx)
            .await
            .map_err(db_err)?;
    sqlx::query(
        r#"
        UPDATE activity_events
        SET subject_title = $1, summary = $2, payload = '{}'::jsonb
        WHERE subject_type = 'bookmark' AND subject_id = $3
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(SCRUBBED_SUMMARY)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    sqlx::query(
        r#"
        UPDATE notifications
        SET subject_title = $1, body = $2, payload = '{}'::jsonb
        WHERE subject_type = 'bookmark' AND subject_id = $3
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(SCRUBBED_SUMMARY)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(tag_ids)
}

/// Remove what still names moved items in plaintext once they are in a vault: tags no
/// bookmark uses any more, likes, comments and ratings, moderation audit titles, and list
/// activity and notifications.
async fn scrub_moved_in(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    bookmark_ids: &[Uuid],
    category_ids: &[Uuid],
    tag_ids: &[Uuid],
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM tags t
        WHERE t.owner_user_id = $1 AND t.id = ANY($2)
          AND NOT EXISTS (SELECT 1 FROM bookmark_tags bt WHERE bt.tag_id = t.id)
        "#,
    )
    .bind(user_id)
    .bind(tag_ids)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    for table in ["social_likes", "social_comments", "social_ratings"] {
        // `table` comes from the fixed list above.
        sqlx::query(&format!(
            r#"
            DELETE FROM {table}
            WHERE (subject_type = 'bookmark' AND subject_id = ANY($1))
               OR (subject_type = 'category' AND subject_id = ANY($2))
            "#
        ))
        .bind(bookmark_ids)
        .bind(category_ids)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    }

    sqlx::query(
        r#"
        UPDATE moderation_actions SET subject_title = $1
        WHERE subject_id = ANY($2) OR subject_id = ANY($3)
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(bookmark_ids)
    .bind(category_ids)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    sqlx::query(
        r#"
        UPDATE activity_events
        SET subject_title = $1, summary = $2, payload = '{}'::jsonb
        WHERE subject_type = 'category' AND subject_id = ANY($3)
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(SCRUBBED_SUMMARY)
    .bind(category_ids)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    sqlx::query(
        r#"
        UPDATE notifications
        SET subject_title = $1, body = $2, payload = '{}'::jsonb
        WHERE subject_type = 'category' AND subject_id = ANY($3)
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(SCRUBBED_SUMMARY)
    .bind(category_ids)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn write_plain(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    payload: &VaultPayload,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET is_vault = FALSE, vault_id = NULL, vault_ciphertext = NULL, vault_nonce = NULL,
            url = $1, title = $2, description = $3, notes = $4, updated_at = now()
        WHERE id = $5
        "#,
    )
    .bind(&payload.url)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.notes)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

pub async fn move_to_vault(
    pool: &PgPool,
    user_id: Uuid,
    req: MoveToVaultRequest,
    keyring: &VaultKeyring,
) -> Result<VaultMoveOutcome> {
    let destination = req.destination_vault_id;
    if let Some(id) = destination {
        get_user_vault(pool, user_id, id).await?;
    }
    let categories = expand_categories(pool, user_id, &req.category_ids).await?;
    let category_ids: Vec<Uuid> = categories.iter().map(|c| c.id).collect();
    let bookmarks = collect_bookmarks(pool, user_id, &req.bookmark_ids, &category_ids).await?;

    if let Some(id) = destination {
        if !keyring.contains(id) {
            return Err(Error::VaultLocked);
        }
    }

    // Items already at the destination stay put.
    let bookmarks: Vec<Bookmark> = bookmarks
        .into_iter()
        .filter(|b| b.vault_id != destination)
        .collect();

    let mut loaded = Vec::with_capacity(bookmarks.len());
    for bookmark in bookmarks {
        let payload = read_payload(pool, &bookmark, keyring).await?;
        loaded.push((bookmark, payload));
    }

    let blockers = find_blockers(pool, &categories, &loaded, destination.is_some()).await?;
    if !blockers.is_empty() {
        return Ok(VaultMoveOutcome::Blocked(blockers));
    }

    // Keep the transaction short: encrypt before it opens.
    let mut encrypted = Vec::new();
    if let Some(id) = destination {
        let key = *keyring.get(id).ok_or(Error::VaultLocked)?;
        for (bookmark, payload) in &loaded {
            let payload = payload.clone();
            let (ct, nonce) =
                super::blocking::run_blocking(move || crypto::encrypt_payload(&key, &payload))
                    .await?;
            encrypted.push((bookmark.id, ct, nonce));
        }
    }

    let mut tag_ids: Vec<(Uuid, Vec<Uuid>)> = Vec::new();
    if destination.is_none() {
        for (bookmark, payload) in &loaded {
            let tags = ensure_tags(pool, user_id, &payload.tags).await?;
            tag_ids.push((bookmark.id, tags.into_iter().map(|t| t.id).collect()));
        }
    }

    let mut tx = pool.begin().await.map_err(db_err)?;

    for category in &categories {
        let keeps_parent = category
            .parent_id
            .map(|parent| category_ids.contains(&parent))
            .unwrap_or(false);
        sqlx::query(
            r#"
            UPDATE categories
            SET is_vault = $1, vault_id = $2,
                parent_id = CASE WHEN $3 THEN parent_id END,
                updated_at = now()
            WHERE id = $4
            "#,
        )
        .bind(destination.is_some())
        .bind(destination)
        .bind(keeps_parent)
        .bind(category.id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }

    let mut freed_tags = Vec::new();
    let mut moved_in = Vec::new();
    for (id, ct, nonce) in encrypted {
        let vault_id = destination.ok_or(Error::msg("move destination missing"))?;
        freed_tags.extend(write_encrypted(&mut tx, id, vault_id, ct, nonce).await?);
        moved_in.push(id);
    }
    if destination.is_some() {
        scrub_moved_in(&mut tx, user_id, &moved_in, &category_ids, &freed_tags).await?;
    }
    if destination.is_none() {
        for (bookmark, payload) in &loaded {
            write_plain(&mut tx, bookmark.id, payload).await?;
        }
        for (id, tags) in &tag_ids {
            for tag_id in tags {
                sqlx::query(
                    "INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
        }
    }

    tx.commit().await.map_err(db_err)?;

    let was_plain = |b: &Bookmark| !b.is_vault;
    let left_search = if destination.is_some() {
        loaded
            .iter()
            .filter(|(b, _)| was_plain(b))
            .map(|(b, _)| b.id)
            .collect()
    } else {
        Vec::new()
    };
    let joined_search = if destination.is_none() {
        loaded
            .iter()
            .filter(|(b, _)| !was_plain(b))
            .map(|(b, _)| b.id)
            .collect()
    } else {
        Vec::new()
    };
    Ok(VaultMoveOutcome::Moved {
        left_search,
        joined_search,
    })
}
