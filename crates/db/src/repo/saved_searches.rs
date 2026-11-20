//! Named library filter snapshots.

use linkdean_core::{
    CreateSavedSearch, Error, PatchSavedSearch, Result, SavedSearch, SavedSearchFilters,
    SavedSearchView,
};
use sqlx::PgPool;
use uuid::Uuid;

const MAX_PER_USER: i64 = 50;
const MAX_NAME: usize = 80;

const SORT_KEYS: &[&str] = &[
    "updated_desc",
    "updated_asc",
    "title_asc",
    "title_desc",
    "created_desc",
];
const FLAG_KEYS: &[&str] = &["unread", "favorite", "pinned"];
const SINCE_KEYS: &[&str] = &["7d", "30d", "90d", "year"];

fn to_view(row: SavedSearch) -> Result<SavedSearchView> {
    let filters: SavedSearchFilters =
        serde_json::from_value(row.filters).map_err(|e| Error::msg(e.to_string()))?;
    Ok(SavedSearchView {
        id: row.id,
        name: row.name,
        filters,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn normalize_name(raw: &str) -> Result<String> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(Error::BadRequest("name required".into()));
    }
    if name.len() > MAX_NAME {
        return Err(Error::BadRequest(format!("name max {MAX_NAME} chars")));
    }
    Ok(name)
}

/// Drop empty / invalid filter fields; reject unknown flag/sort/since values.
pub fn normalize_filters(raw: SavedSearchFilters) -> Result<SavedSearchFilters> {
    let q = raw
        .q
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let sort = match raw.sort.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) if SORT_KEYS.contains(&s) => {
            if s == "updated_desc" {
                None
            } else {
                Some(s.to_string())
            }
        }
        Some(_) => return Err(Error::BadRequest("unknown sort".into())),
    };
    let category: Vec<String> = raw
        .category
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let tag: Vec<String> = raw
        .tag
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut flag = Vec::new();
    for f in raw.flag {
        let t = f.trim();
        if t.is_empty() {
            continue;
        }
        if !FLAG_KEYS.contains(&t) {
            return Err(Error::BadRequest(format!("unknown flag `{t}`")));
        }
        if !flag.iter().any(|x: &String| x == t) {
            flag.push(t.to_string());
        }
    }
    let since = match raw
        .since
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(s) if SINCE_KEYS.contains(&s) => Some(s.to_string()),
        Some(_) => return Err(Error::BadRequest("unknown since".into())),
    };
    let vault = raw.vault.filter(|v| *v);
    let vault_id = raw.vault_id;
    let folder = raw
        .folder
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "all");

    Ok(SavedSearchFilters {
        q,
        sort,
        category,
        tag,
        flag,
        since,
        vault,
        vault_id,
        folder,
    })
}

fn map_db_err(e: sqlx::Error) -> Error {
    let msg = e.to_string();
    if msg.contains("saved_searches_owner_name_unique")
        || msg.contains("duplicate key")
        || msg.contains("unique")
    {
        return Error::Conflict("a saved search with that name already exists".into());
    }
    Error::msg(msg)
}

pub async fn list_saved_searches(pool: &PgPool, owner: Uuid) -> Result<Vec<SavedSearchView>> {
    let rows = sqlx::query_as::<_, SavedSearch>(
        r#"
        SELECT *
        FROM saved_searches
        WHERE owner_user_id = $1
        ORDER BY lower(name)
        "#,
    )
    .bind(owner)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    rows.into_iter().map(to_view).collect()
}

pub async fn get_saved_search(pool: &PgPool, owner: Uuid, id: Uuid) -> Result<SavedSearchView> {
    let row = sqlx::query_as::<_, SavedSearch>(
        r#"
        SELECT *
        FROM saved_searches
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
    to_view(row)
}

pub async fn create_saved_search(
    pool: &PgPool,
    owner: Uuid,
    req: CreateSavedSearch,
) -> Result<SavedSearchView> {
    let name = normalize_name(&req.name)?;
    let filters = normalize_filters(req.filters)?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM saved_searches WHERE owner_user_id = $1")
            .bind(owner)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    if count >= MAX_PER_USER {
        return Err(Error::BadRequest(format!(
            "at most {MAX_PER_USER} saved searches"
        )));
    }
    let filters_json = serde_json::to_value(&filters).map_err(|e| Error::msg(e.to_string()))?;
    let row = sqlx::query_as::<_, SavedSearch>(
        r#"
        INSERT INTO saved_searches (owner_user_id, name, filters)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(owner)
    .bind(&name)
    .bind(&filters_json)
    .fetch_one(pool)
    .await
    .map_err(map_db_err)?;
    to_view(row)
}

pub async fn patch_saved_search(
    pool: &PgPool,
    owner: Uuid,
    id: Uuid,
    req: PatchSavedSearch,
) -> Result<SavedSearchView> {
    if req.name.is_none() && req.filters.is_none() {
        return Err(Error::BadRequest("nothing to update".into()));
    }
    let existing = sqlx::query_as::<_, SavedSearch>(
        r#"
        SELECT *
        FROM saved_searches
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    let name = match req.name {
        Some(n) => normalize_name(&n)?,
        None => existing.name.clone(),
    };
    let filters = match req.filters {
        Some(f) => normalize_filters(f)?,
        None => serde_json::from_value(existing.filters.clone())
            .map_err(|e| Error::msg(e.to_string()))?,
    };
    let filters_json = serde_json::to_value(&filters).map_err(|e| Error::msg(e.to_string()))?;

    let row = sqlx::query_as::<_, SavedSearch>(
        r#"
        UPDATE saved_searches
        SET name = $3, filters = $4, updated_at = now()
        WHERE id = $1 AND owner_user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(owner)
    .bind(&name)
    .bind(&filters_json)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)?
    .ok_or(Error::NotFound)?;
    to_view(row)
}

pub async fn delete_saved_search(pool: &PgPool, owner: Uuid, id: Uuid) -> Result<()> {
    let res = sqlx::query(
        r#"
        DELETE FROM saved_searches
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(id)
    .bind(owner)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_rejects_bad_flags() {
        let ok = normalize_filters(SavedSearchFilters {
            q: Some("  hello  ".into()),
            sort: Some("updated_desc".into()),
            flag: vec!["favorite".into()],
            folder: Some("all".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(ok.q.as_deref(), Some("hello"));
        assert!(ok.sort.is_none());
        assert_eq!(ok.flag, vec!["favorite"]);
        assert!(ok.folder.is_none());

        assert!(normalize_filters(SavedSearchFilters {
            flag: vec!["nope".into()],
            ..Default::default()
        })
        .is_err());
    }
}
