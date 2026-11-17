//! Archived copies of bookmarks: the queue the archive runner works through, and the record of
//! each stored copy. Files live on disk; the caller deletes the names these functions return.

use super::platform::get_platform_settings;
use super::quotas::METER_ARCHIVE;
use chrono::{DateTime, Utc};
use linkdean_core::{
    clean_archive_formats, ArchiveFormat, ArchiveStatus, ArchiveSummary, BookmarkArchiveView,
    Error, Result, VaultKeyring,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Shown on an automatic archive the quota had no room for.
pub const ARCHIVE_STORAGE_FULL: &str = "Archive storage is full";

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArchiveRow {
    pub bookmark_id: Uuid,
    pub format: String,
    pub status: String,
    pub file_name: Option<String>,
    pub bytes: i64,
    pub content_type: Option<String>,
    pub final_url: Option<String>,
    pub key_vault_id: Option<Uuid>,
    pub nonce: Option<Vec<u8>>,
    pub captured_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl ArchiveRow {
    pub fn format(&self) -> Result<ArchiveFormat> {
        ArchiveFormat::parse(&self.format)
    }

    fn view(&self) -> Result<BookmarkArchiveView> {
        Ok(BookmarkArchiveView {
            format: self.format()?,
            status: ArchiveStatus::parse(&self.status)?,
            bytes: if self.file_name.is_some() {
                self.bytes
            } else {
                0
            },
            content_type: self.content_type.clone(),
            captured_at: self.captured_at,
            error: self.error.clone(),
            available: self.file_name.is_some(),
        })
    }
}

const ROW_COLS: &str = "bookmark_id, format, status, file_name, bytes, content_type, final_url, \
                        key_vault_id, nonce, captured_at, error";

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

fn names(formats: &[ArchiveFormat]) -> Vec<String> {
    formats.iter().map(|f| f.as_str().to_string()).collect()
}

/// Queue `formats` of a bookmark the user may write. Refuses a trashed bookmark, a vault
/// bookmark whose vault the caller has not unlocked, and an archive while archiving is off or
/// the owner has no archive storage left.
pub async fn request_archive(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    formats: &[ArchiveFormat],
    keyring: &VaultKeyring,
) -> Result<Vec<BookmarkArchiveView>> {
    let formats = clean_archive_formats(formats)?;
    let b = super::bookmarks::get_bookmark_raw(pool, user_id, bookmark_id).await?;
    if !super::shares::bookmark_can_write(pool, user_id, bookmark_id).await? {
        return Err(Error::Forbidden);
    }
    if b.deleted_at.is_some() {
        return Err(Error::BadRequest(
            "Restore the bookmark from Trash to archive it.".into(),
        ));
    }
    if b.is_vault && !b.vault_id.is_some_and(|v| keyring.get(v).is_some()) {
        return Err(Error::VaultLocked);
    }
    let settings = get_platform_settings(pool).await?;
    if !settings.archives_enabled {
        return Err(Error::BadRequest(
            "Archiving is turned off on this server.".into(),
        ));
    }
    if let Some(limit) = settings.max_archive_bytes {
        let used = archive_bytes_for_owner(pool, b.owner_user_id).await?;
        if used >= limit {
            return Err(Error::QuotaExceeded {
                meter: METER_ARCHIVE,
                limit,
                used,
            });
        }
    }
    queue(pool, bookmark_id, user_id, &formats).await?;
    list_archives(pool, bookmark_id).await
}

/// Archive a bookmark its creator just saved, when they turned that on. Returns whether
/// anything was queued. With no storage left the rows are marked failed, so the drawer says why.
pub async fn archive_new_bookmark(pool: &PgPool, user_id: Uuid, bookmark_id: Uuid) -> Result<bool> {
    let user = super::users::get_user_settings(pool, user_id).await?;
    if !user.archive_new {
        return Ok(false);
    }
    let settings = get_platform_settings(pool).await?;
    if !settings.archives_enabled {
        return Ok(false);
    }
    let formats: Vec<ArchiveFormat> = user
        .archive_formats
        .iter()
        .filter_map(|f| ArchiveFormat::parse(f).ok())
        .collect();
    let formats = clean_archive_formats(&formats)?;
    if let Some(limit) = settings.max_archive_bytes {
        if archive_bytes_for_owner(pool, user_id).await? >= limit {
            sqlx::query(
                r#"
                INSERT INTO bookmark_archives (bookmark_id, format, status, requested_by, error)
                SELECT $1, f, 'failed', $2, $4 FROM unnest($3::text[]) f
                ON CONFLICT (bookmark_id, format) DO NOTHING
                "#,
            )
            .bind(bookmark_id)
            .bind(user_id)
            .bind(names(&formats))
            .bind(ARCHIVE_STORAGE_FULL)
            .execute(pool)
            .await
            .map_err(db_err)?;
            return Ok(false);
        }
    }
    queue(pool, bookmark_id, user_id, &formats).await?;
    Ok(true)
}

/// Rows already being captured stay as they are. The old copy stays until a new one replaces it.
async fn queue(
    pool: &PgPool,
    bookmark_id: Uuid,
    user_id: Uuid,
    formats: &[ArchiveFormat],
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO bookmark_archives (bookmark_id, format, status, requested_by)
        SELECT $1, f, 'queued', $2 FROM unnest($3::text[]) f
        ON CONFLICT (bookmark_id, format) DO UPDATE
        SET status = 'queued', error = NULL, queued_at = now(), started_at = NULL,
            requested_by = EXCLUDED.requested_by
        WHERE bookmark_archives.status <> 'running'
        "#,
    )
    .bind(bookmark_id)
    .bind(user_id)
    .bind(names(formats))
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Every copy of a bookmark. The caller checks read access.
pub async fn list_archives(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<BookmarkArchiveView>> {
    let rows = sqlx::query_as::<_, ArchiveRow>(sqlx::AssertSqlSafe(format!(
        "SELECT {ROW_COLS} FROM bookmark_archives WHERE bookmark_id = $1 ORDER BY format"
    )))
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    rows.iter().map(ArchiveRow::view).collect()
}

/// One copy, for serving. The caller checks read access.
pub async fn archive_row(
    pool: &PgPool,
    bookmark_id: Uuid,
    format: ArchiveFormat,
) -> Result<Option<ArchiveRow>> {
    sqlx::query_as::<_, ArchiveRow>(sqlx::AssertSqlSafe(format!(
        "SELECT {ROW_COLS} FROM bookmark_archives WHERE bookmark_id = $1 AND format = $2"
    )))
    .bind(bookmark_id)
    .bind(format.as_str())
    .fetch_optional(pool)
    .await
    .map_err(db_err)
}

/// Badge data for a page of bookmarks.
pub async fn archive_summaries(
    pool: &PgPool,
    bookmark_ids: &[Uuid],
) -> Result<HashMap<Uuid, ArchiveSummary>> {
    if bookmark_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, ArchiveRow>(sqlx::AssertSqlSafe(format!(
        "SELECT {ROW_COLS} FROM bookmark_archives WHERE bookmark_id = ANY($1) ORDER BY format"
    )))
    .bind(bookmark_ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let mut out: HashMap<Uuid, ArchiveSummary> = HashMap::new();
    for row in rows {
        let view = row.view()?;
        let s = out.entry(row.bookmark_id).or_default();
        let pending = matches!(view.status, ArchiveStatus::Queued | ArchiveStatus::Running);
        if view.available {
            s.formats.push(view.format);
            s.captured_at = s.captured_at.max(view.captured_at);
        }
        s.pending |= pending;
        s.failed |= !pending && view.error.is_some();
    }
    Ok(out)
}

/// Claim the oldest queued bookmark: all its queued formats go to running together, so one
/// visit to the page makes every format.
pub async fn claim_next_archive(pool: &PgPool) -> Result<Option<(Uuid, Vec<ArchiveFormat>)>> {
    // shortcut: one runner claims at a time; with several API processes two could split one
    // bookmark's formats between them, which only costs a second Chrome visit.
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        WITH next AS (
            SELECT bookmark_id FROM bookmark_archives
            WHERE status = 'queued'
            ORDER BY queued_at
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE bookmark_archives a
        SET status = 'running', started_at = now()
        FROM next
        WHERE a.bookmark_id = next.bookmark_id AND a.status = 'queued'
        RETURNING a.bookmark_id, a.format
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let Some(bookmark_id) = rows.first().map(|r| r.0) else {
        return Ok(None);
    };
    let mut formats = rows
        .iter()
        .map(|(_, f)| ArchiveFormat::parse(f))
        .collect::<Result<Vec<_>>>()?;
    formats.sort();
    Ok(Some((bookmark_id, formats)))
}

/// The bookmark a claimed archive belongs to, without an access check: the runner works for
/// whoever queued it.
pub async fn archive_target(
    pool: &PgPool,
    bookmark_id: Uuid,
) -> Result<Option<linkdean_core::Bookmark>> {
    sqlx::query_as::<_, linkdean_core::Bookmark>("SELECT * FROM bookmarks WHERE id = $1")
        .bind(bookmark_id)
        .fetch_optional(pool)
        .await
        .map_err(db_err)
}

/// Every stored file name per bookmark, for the sweep that removes files nothing points at.
pub async fn archive_file_names(pool: &PgPool) -> Result<HashMap<Uuid, Vec<String>>> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT bookmark_id, file_name FROM bookmark_archives WHERE file_name IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let mut out: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, name) in rows {
        out.entry(id).or_default().push(name);
    }
    Ok(out)
}

/// A copy written to disk, ready to record.
#[derive(Debug, Clone)]
pub struct StoredCopy {
    pub format: ArchiveFormat,
    pub file_name: String,
    pub bytes: i64,
    pub content_type: String,
    pub final_url: Option<String>,
    pub key_vault_id: Option<Uuid>,
    pub nonce: Option<Vec<u8>>,
}

/// Record new copies of a bookmark being captured. Returns the files no longer used: the copies
/// these replace, or the new files themselves when the archive was deleted meanwhile. A `file`
/// copy replaces the html and pdf rows, and an html or pdf copy replaces a `file` row.
pub async fn finish_archive(
    pool: &PgPool,
    bookmark_id: Uuid,
    copies: &[StoredCopy],
) -> Result<Vec<String>> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT format, status, file_name FROM bookmark_archives WHERE bookmark_id = $1 FOR UPDATE",
    )
    .bind(bookmark_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db_err)?;
    if !rows.iter().any(|(_, status, _)| status == "running") {
        return Ok(copies.iter().map(|c| c.file_name.clone()).collect());
    }
    let is_file = copies.iter().any(|c| c.format == ArchiveFormat::File);
    let mut unused = Vec::new();
    for (format, _, file_name) in &rows {
        let replaced = copies.iter().any(|c| c.format.as_str() == format)
            || (is_file && format != "file")
            || (!is_file && !copies.is_empty() && format == "file");
        if replaced {
            unused.extend(file_name.clone());
        }
    }
    if is_file {
        sqlx::query("DELETE FROM bookmark_archives WHERE bookmark_id = $1 AND format <> 'file'")
            .bind(bookmark_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
    } else if !copies.is_empty() {
        sqlx::query("DELETE FROM bookmark_archives WHERE bookmark_id = $1 AND format = 'file'")
            .bind(bookmark_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
    }
    for c in copies {
        sqlx::query(
            r#"
            INSERT INTO bookmark_archives (bookmark_id, format, status, file_name, bytes,
                content_type, final_url, key_vault_id, nonce, captured_at)
            VALUES ($1, $2, 'done', $3, $4, $5, $6, $7, $8, now())
            ON CONFLICT (bookmark_id, format) DO UPDATE
            SET status = 'done', file_name = $3, bytes = $4, content_type = $5, final_url = $6,
                key_vault_id = $7, nonce = $8, captured_at = now(), error = NULL,
                started_at = NULL
            "#,
        )
        .bind(bookmark_id)
        .bind(c.format.as_str())
        .bind(&c.file_name)
        .bind(c.bytes)
        .bind(&c.content_type)
        .bind(&c.final_url)
        .bind(c.key_vault_id)
        .bind(&c.nonce)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    unused.retain(|old| !copies.iter().any(|c| &c.file_name == old));
    Ok(unused)
}

/// Mark running formats failed. One with an earlier copy keeps it: back to done, with the error.
pub async fn fail_archive(
    pool: &PgPool,
    bookmark_id: Uuid,
    formats: &[ArchiveFormat],
    error: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmark_archives
        SET status = CASE WHEN file_name IS NULL THEN 'failed' ELSE 'done' END,
            error = $3, started_at = NULL
        WHERE bookmark_id = $1 AND format = ANY($2) AND status = 'running'
        "#,
    )
    .bind(bookmark_id)
    .bind(names(formats))
    .bind(error)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// At start-up nothing is being captured yet, so running rows were cut off by a restart.
pub async fn requeue_running_archives(pool: &PgPool) -> Result<u64> {
    let done = sqlx::query(
        "UPDATE bookmark_archives SET status = 'queued', started_at = NULL WHERE status = 'running'",
    )
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(done.rows_affected())
}

/// Delete every copy of a bookmark the user may write. Returns the files to remove.
pub async fn delete_archives(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
) -> Result<Vec<String>> {
    super::bookmarks::get_bookmark_raw(pool, user_id, bookmark_id).await?;
    if !super::shares::bookmark_can_write(pool, user_id, bookmark_id).await? {
        return Err(Error::Forbidden);
    }
    let names: Vec<Option<String>> = sqlx::query_scalar(
        "DELETE FROM bookmark_archives WHERE bookmark_id = $1 RETURNING file_name",
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    Ok(names.into_iter().flatten().collect())
}

/// Stored copies on the bookmarks a person owns, Trash included.
pub async fn archive_bytes_for_owner(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(a.bytes), 0)::bigint
        FROM bookmark_archives a
        JOIN bookmarks b ON b.id = a.bookmark_id
        WHERE b.owner_user_id = $1 AND a.file_name IS NOT NULL
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(db_err)
}

/// The owner's copies encrypted for another vault than their bookmark is in now (or for none),
/// after a move in, out of or between vaults. Pairs each row with the vault it should be
/// encrypted for.
pub async fn archives_needing_rekey(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<Vec<(ArchiveRow, Option<Uuid>)>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        row: ArchiveRow,
        target_vault_id: Option<Uuid>,
    }
    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT a.bookmark_id, a.format, a.status, a.file_name, a.bytes, a.content_type,
               a.final_url, a.key_vault_id, a.nonce, a.captured_at, a.error,
               b.vault_id AS target_vault_id
        FROM bookmark_archives a
        JOIN bookmarks b ON b.id = a.bookmark_id
        WHERE b.owner_user_id = $1
          AND a.file_name IS NOT NULL
          AND a.key_vault_id IS DISTINCT FROM b.vault_id
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    Ok(rows
        .into_iter()
        .map(|r| (r.row, r.target_vault_id))
        .collect())
}

/// Point a copy at its re-encrypted file, if it still holds `old_file`. Returns whether it did;
/// when not, the caller deletes the new file instead of the old one.
pub async fn set_archive_key(
    pool: &PgPool,
    bookmark_id: Uuid,
    format: ArchiveFormat,
    old_file: &str,
    new_file: &str,
    key_vault_id: Option<Uuid>,
    nonce: Option<Vec<u8>>,
) -> Result<bool> {
    let done = sqlx::query(
        r#"
        UPDATE bookmark_archives
        SET file_name = $4, key_vault_id = $5, nonce = $6,
            final_url = CASE WHEN $5::uuid IS NULL THEN final_url ELSE NULL END
        WHERE bookmark_id = $1 AND format = $2 AND file_name = $3
        "#,
    )
    .bind(bookmark_id)
    .bind(format.as_str())
    .bind(old_file)
    .bind(new_file)
    .bind(key_vault_id)
    .bind(nonce)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(done.rows_affected() == 1)
}
