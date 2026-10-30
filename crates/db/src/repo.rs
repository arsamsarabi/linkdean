use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_password, hash_token, verify_password};
use linkdean_core::{
    Bookmark, BookmarkView, Category, CategoryRef, CreateBookmark, CreateCategory, Error, ImportJob,
    Result,
    Tag, UpdateBookmark, UpdateCategory, UpdateProfile, UpdateSettings, User, UserPublic,
    UserSettings, Vault, VaultPayload,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn count_users(pool: &PgPool) -> Result<i64> {
    let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.0)
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password: Option<&str>,
    display_name: Option<&str>,
    oidc_subject: Option<&str>,
    oidc_issuer: Option<&str>,
) -> Result<User> {
    let password_hash = match password {
        Some(p) => Some(hash_password(p)?),
        None => None,
    };
    let is_first = count_users(pool).await? == 0;
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (
            email, display_name, password_hash, oidc_subject, oidc_issuer,
            is_admin, onboarding_completed
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(email.to_ascii_lowercase())
    .bind(display_name)
    .bind(password_hash)
    .bind(oidc_subject)
    .bind(oidc_issuer)
    .bind(is_first) // first user = admin
    .bind(false) // onboarding starts incomplete for new admins
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
            Error::Conflict("email already registered".into())
        } else {
            Error::msg(e.to_string())
        }
    })?;
    ensure_user_settings(pool, user.id).await?;
    Ok(user)
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email.to_ascii_lowercase())
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn find_user_by_oidc(
    pool: &PgPool,
    issuer: &str,
    subject: &str,
) -> Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE oidc_issuer = $1 AND oidc_subject = $2",
    )
    .bind(issuer)
    .bind(subject)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn user_has_vault(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vaults WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.0 > 0)
}

pub async fn to_public(pool: &PgPool, user: &User) -> Result<UserPublic> {
    Ok(UserPublic {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        bio: user.bio.clone(),
        timezone: user.timezone.clone(),
        has_avatar: user.has_avatar,
        has_password: user.password_hash.is_some(),
        has_vault: user_has_vault(pool, user.id).await?,
        is_admin: user.is_admin,
        onboarding_completed: user.onboarding_completed,
    })
}

pub async fn ensure_user_settings(pool: &PgPool, user_id: Uuid) -> Result<UserSettings> {
    sqlx::query_as::<_, UserSettings>(
        r#"
        INSERT INTO user_settings (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_user_settings(pool: &PgPool, user_id: Uuid) -> Result<UserSettings> {
    if let Some(s) = sqlx::query_as::<_, UserSettings>(
        "SELECT * FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    {
        return Ok(s);
    }
    ensure_user_settings(pool, user_id).await
}

pub async fn update_user_settings(
    pool: &PgPool,
    user_id: Uuid,
    patch: &UpdateSettings,
) -> Result<UserSettings> {
    let mut s = get_user_settings(pool, user_id).await?;
    if let Some(v) = patch.show_hidden {
        s.show_hidden = v;
    }
    if let Some(ref v) = patch.vault_search_mode {
        match v.as_str() {
            "never" | "when_unlocked" | "always" => s.vault_search_mode = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "vault_search_mode must be never|when_unlocked|always".into(),
                ))
            }
        }
    }
    if let Some(ref v) = patch.theme {
        match v.as_str() {
            "light" | "dark" | "system" => s.theme = v.clone(),
            _ => return Err(Error::BadRequest("theme must be light|dark|system".into())),
        }
    }
    if let Some(v) = patch.vault_auto_lock_minutes {
        if v < 0 {
            return Err(Error::BadRequest("vault_auto_lock_minutes must be >= 0".into()));
        }
        s.vault_auto_lock_minutes = v;
    }
    if let Some(ref v) = patch.default_landing {
        match v.as_str() {
            "/" | "/vault" | "/import" => s.default_landing = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "default_landing must be /|/vault|/import".into(),
                ))
            }
        }
    }
    sqlx::query_as::<_, UserSettings>(
        r#"
        UPDATE user_settings SET
            show_hidden = $2,
            vault_search_mode = $3,
            theme = $4,
            vault_auto_lock_minutes = $5,
            default_landing = $6,
            updated_at = now()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(s.show_hidden)
    .bind(&s.vault_search_mode)
    .bind(&s.theme)
    .bind(s.vault_auto_lock_minutes)
    .bind(&s.default_landing)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn update_profile(pool: &PgPool, user_id: Uuid, patch: &UpdateProfile) -> Result<User> {
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;

    let display_name = match &patch.display_name {
        Some(v) => v.clone(),
        None => user.display_name.clone(),
    };
    let bio = match &patch.bio {
        Some(v) => v.clone(),
        None => user.bio.clone(),
    };
    let timezone = match &patch.timezone {
        Some(v) => v.clone(),
        None => user.timezone.clone(),
    };
    let email = match &patch.email {
        Some(e) => {
            let e = e.trim().to_ascii_lowercase();
            if e.is_empty() {
                return Err(Error::BadRequest("email required".into()));
            }
            if e != user.email {
                if find_user_by_email(pool, &e).await?.is_some() {
                    return Err(Error::Conflict("email already registered".into()));
                }
            }
            e
        }
        None => user.email.clone(),
    };

    sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET
            display_name = $2,
            bio = $3,
            timezone = $4,
            email = $5,
            updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(display_name)
    .bind(bio)
    .bind(timezone)
    .bind(email)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn change_password(
    pool: &PgPool,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<()> {
    if new_password.len() < 8 {
        return Err(Error::BadRequest("password min 8 chars".into()));
    }
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let hash = user
        .password_hash
        .as_deref()
        .ok_or_else(|| Error::BadRequest("account has no local password".into()))?;
    if !verify_password(current_password, hash)? {
        return Err(Error::Unauthorized);
    }
    let new_hash = hash_password(new_password)?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .bind(new_hash)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn set_has_avatar(pool: &PgPool, user_id: Uuid, has_avatar: bool) -> Result<User> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET has_avatar = $2, updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(has_avatar)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn complete_onboarding(pool: &PgPool, user_id: Uuid) -> Result<User> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET onboarding_completed = TRUE, updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn authenticate_local(pool: &PgPool, email: &str, password: &str) -> Result<User> {
    let user = find_user_by_email(pool, email)
        .await?
        .ok_or(Error::Unauthorized)?;
    let hash = user.password_hash.as_deref().ok_or(Error::Unauthorized)?;
    if !verify_password(password, hash)? {
        return Err(Error::Unauthorized);
    }
    Ok(user)
}

pub async fn create_session(pool: &PgPool, user_id: Uuid, days: i64) -> Result<String> {
    let token = hex::encode(crypto::random_bytes(32));
    let token_hash = hash_token(&token);
    let expires = Utc::now() + Duration::days(days);
    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(token)
}

pub async fn user_from_token(pool: &PgPool, token: &str) -> Result<Option<User>> {
    let token_hash = hash_token(token);
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT user_id FROM sessions
        WHERE token_hash = $1 AND expires_at > now()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    match row {
        Some((uid,)) => find_user_by_id(pool, uid).await,
        None => Ok(None),
    }
}

pub async fn delete_session(pool: &PgPool, token: &str) -> Result<()> {
    let token_hash = hash_token(token);
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn list_categories(
    pool: &PgPool,
    user_id: Uuid,
    include_hidden: bool,
    include_vault: bool,
) -> Result<Vec<Category>> {
    sqlx::query_as::<_, Category>(
        r#"
        SELECT * FROM categories
        WHERE owner_user_id = $1
          AND ($2 OR is_hidden = FALSE)
          AND ($3 OR is_vault = FALSE)
        ORDER BY position, name
        "#,
    )
    .bind(user_id)
    .bind(include_hidden)
    .bind(include_vault)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Category> {
    sqlx::query_as::<_, Category>(
        "SELECT * FROM categories WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

pub async fn create_category(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateCategory,
) -> Result<Category> {
    if let Some(pid) = req.parent_id {
        let parent = get_category(pool, user_id, pid).await?;
        if parent.is_vault && !req.is_vault {
            return Err(Error::BadRequest(
                "non-vault category cannot nest under vault".into(),
            ));
        }
    }
    sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (owner_user_id, parent_id, name, is_hidden, is_vault)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.parent_id)
    .bind(req.name)
    .bind(req.is_hidden)
    .bind(req.is_vault)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn update_category(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateCategory,
) -> Result<Category> {
    let mut cat = get_category(pool, user_id, id).await?;
    if let Some(name) = req.name {
        cat.name = name;
    }
    if let Some(parent) = req.parent_id {
        cat.parent_id = parent;
    }
    if let Some(h) = req.is_hidden {
        cat.is_hidden = h;
    }
    if let Some(p) = req.position {
        cat.position = p;
    }
    sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET name = $1, parent_id = $2, is_hidden = $3, position = $4, updated_at = now()
        WHERE id = $5 AND owner_user_id = $6
        RETURNING *
        "#,
    )
    .bind(&cat.name)
    .bind(cat.parent_id)
    .bind(cat.is_hidden)
    .bind(cat.position)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn delete_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let res = sqlx::query("DELETE FROM categories WHERE id = $1 AND owner_user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn list_tags(pool: &PgPool, user_id: Uuid) -> Result<Vec<Tag>> {
    sqlx::query_as::<_, Tag>(
        "SELECT * FROM tags WHERE owner_user_id = $1 ORDER BY name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn ensure_tags(pool: &PgPool, user_id: Uuid, names: &[String]) -> Result<Vec<Tag>> {
    let mut out = Vec::new();
    for name in names {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (owner_user_id, name)
            VALUES ($1, $2)
            ON CONFLICT (owner_user_id, name) DO UPDATE SET name = EXCLUDED.name
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        out.push(tag);
    }
    Ok(out)
}

async fn set_bookmark_tags(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    tags: &[String],
) -> Result<Vec<String>> {
    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = $1")
        .bind(bookmark_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let ensured = ensure_tags(pool, user_id, tags).await?;
    for t in &ensured {
        sqlx::query(
            "INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(bookmark_id)
        .bind(t.id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }
    Ok(ensured.into_iter().map(|t| t.name).collect())
}

async fn tags_for_bookmark(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT t.name FROM tags t
        JOIN bookmark_tags bt ON bt.tag_id = t.id
        WHERE bt.bookmark_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|(n,)| n).collect())
}

fn category_path(cat: &Category, by_id: &HashMap<Uuid, Category>) -> String {
    let mut parts = vec![cat.name.clone()];
    let mut parent = cat.parent_id;
    let mut guard = 0;
    while let Some(pid) = parent {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(p) = by_id.get(&pid) else { break };
        parts.push(p.name.clone());
        parent = p.parent_id;
    }
    parts.reverse();
    parts.join(" / ")
}

async fn categories_for_bookmark(
    pool: &PgPool,
    owner_user_id: Uuid,
    bookmark_id: Uuid,
) -> Result<Vec<CategoryRef>> {
    let linked: Vec<Uuid> = sqlx::query_as(
        r#"
        SELECT category_id FROM bookmark_categories
        WHERE bookmark_id = $1
        ORDER BY category_id
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    if linked.is_empty() {
        return Ok(Vec::new());
    }

    let all = list_categories(pool, owner_user_id, true, true).await?;
    let by_id: HashMap<Uuid, Category> = all.into_iter().map(|c| (c.id, c)).collect();
    let mut out = Vec::new();
    for id in linked {
        if let Some(cat) = by_id.get(&id) {
            out.push(CategoryRef {
                id: cat.id,
                name: cat.name.clone(),
                path: category_path(cat, &by_id),
            });
        }
    }
    Ok(out)
}

async fn set_bookmark_categories(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    category_ids: &[Uuid],
) -> Result<Option<Uuid>> {
    let mut unique = Vec::new();
    for id in category_ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }

    let mut any_vault = false;
    let mut any_non_vault = false;
    for id in &unique {
        let cat = get_category(pool, user_id, *id).await?;
        if cat.is_vault {
            any_vault = true;
        } else {
            any_non_vault = true;
        }
    }
    if any_vault && any_non_vault {
        return Err(Error::BadRequest(
            "cannot mix vault and non-vault categories on one bookmark".into(),
        ));
    }

    sqlx::query("DELETE FROM bookmark_categories WHERE bookmark_id = $1")
        .bind(bookmark_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    for id in &unique {
        sqlx::query(
            "INSERT INTO bookmark_categories (bookmark_id, category_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(bookmark_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }

    let primary = unique.first().copied();
    sqlx::query("UPDATE bookmarks SET category_id = $2 WHERE id = $1")
        .bind(bookmark_id)
        .bind(primary)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(primary)
}

fn resolve_category_ids(req_ids: &[Uuid], legacy: Option<Uuid>) -> Vec<Uuid> {
    if !req_ids.is_empty() {
        return req_ids.to_vec();
    }
    legacy.into_iter().collect()
}

pub async fn bookmark_to_view(
    pool: &PgPool,
    b: Bookmark,
    vault_key: Option<&[u8; 32]>,
) -> Result<Option<BookmarkView>> {
    let categories = categories_for_bookmark(pool, b.owner_user_id, b.id).await?;
    let category_id = categories.first().map(|c| c.id).or(b.category_id);

    if b.is_vault {
        let key = match vault_key {
            Some(k) => k,
            None => return Ok(None),
        };
        let ct = b.vault_ciphertext.as_deref().ok_or(Error::msg("vault blob missing"))?;
        let nonce = b.vault_nonce.as_deref().ok_or(Error::msg("vault nonce missing"))?;
        let payload = crypto::decrypt_payload(key, ct, nonce)?;
        return Ok(Some(BookmarkView {
            id: b.id,
            category_id,
            categories,
            url: payload.url,
            title: payload.title,
            description: payload.description,
            notes: payload.notes,
            favicon_url: b.favicon_url,
            preview_image_url: b.preview_image_url,
            has_preview: b.has_preview,
            is_vault: true,
            tags: payload.tags,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }));
    }
    let tags = tags_for_bookmark(pool, b.id).await?;
    Ok(Some(BookmarkView {
        id: b.id,
        category_id,
        categories,
        url: b.url,
        title: b.title,
        description: b.description,
        notes: b.notes,
        favicon_url: b.favicon_url,
        preview_image_url: b.preview_image_url,
        has_preview: b.has_preview,
        is_vault: false,
        tags,
        created_at: b.created_at,
        updated_at: b.updated_at,
    }))
}

pub async fn list_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Option<Uuid>,
    include_vault: bool,
    vault_key: Option<&[u8; 32]>,
) -> Result<Vec<BookmarkView>> {
    let rows = if let Some(cid) = category_id {
        sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT b.* FROM bookmarks b
            WHERE b.owner_user_id = $1
              AND ($3 OR b.is_vault = FALSE)
              AND (
                b.category_id = $2
                OR EXISTS (
                  SELECT 1 FROM bookmark_categories bc
                  WHERE bc.bookmark_id = b.id AND bc.category_id = $2
                )
              )
            ORDER BY b.updated_at DESC
            "#,
        )
        .bind(user_id)
        .bind(cid)
        .bind(include_vault && vault_key.is_some())
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT * FROM bookmarks
            WHERE owner_user_id = $1
              AND ($2 OR is_vault = FALSE)
            ORDER BY updated_at DESC
            "#,
        )
        .bind(user_id)
        .bind(include_vault && vault_key.is_some())
        .fetch_all(pool)
        .await
    }
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::new();
    for b in rows {
        if let Some(v) = bookmark_to_view(pool, b, vault_key).await? {
            out.push(v);
        }
    }
    Ok(out)
}

pub async fn get_bookmark_raw(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Bookmark> {
    sqlx::query_as::<_, Bookmark>(
        "SELECT * FROM bookmarks WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

pub async fn create_bookmark(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateBookmark,
    vault_key: Option<&[u8; 32]>,
) -> Result<BookmarkView> {
    let cat_ids = resolve_category_ids(&req.category_ids, req.category_id);
    let mut is_vault = false;
    for cid in &cat_ids {
        let cat = get_category(pool, user_id, *cid).await?;
        if cat.is_vault {
            is_vault = true;
        }
    }

    let title = req
        .title
        .clone()
        .unwrap_or_else(|| req.url.clone());
    let primary = cat_ids.first().copied();

    let bookmark = if is_vault {
        let key = vault_key.ok_or(Error::VaultLocked)?;
        let payload = VaultPayload {
            url: req.url.clone(),
            title: title.clone(),
            description: req.description.clone(),
            notes: req.notes.clone(),
            tags: req.tags.clone(),
        };
        let (ct, nonce) = crypto::encrypt_payload(key, &payload)?;
        sqlx::query_as::<_, Bookmark>(
            r#"
            INSERT INTO bookmarks (
                owner_user_id, category_id, url, title, description, notes,
                is_vault, vault_ciphertext, vault_nonce
            ) VALUES ($1,$2,'','','',NULL,TRUE,$3,$4)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(primary)
        .bind(ct)
        .bind(nonce)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
    } else {
        let b = sqlx::query_as::<_, Bookmark>(
            r#"
            INSERT INTO bookmarks (
                owner_user_id, category_id, url, title, description, notes, is_vault
            ) VALUES ($1,$2,$3,$4,$5,$6,FALSE)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(primary)
        .bind(&req.url)
        .bind(&title)
        .bind(&req.description)
        .bind(&req.notes)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        set_bookmark_tags(pool, user_id, b.id, &req.tags).await?;
        b
    };

    set_bookmark_categories(pool, user_id, bookmark.id, &cat_ids).await?;
    let bookmark = get_bookmark_raw(pool, user_id, bookmark.id).await?;

    bookmark_to_view(pool, bookmark, vault_key)
        .await?
        .ok_or(Error::VaultLocked)
}

pub async fn update_bookmark(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateBookmark,
    vault_key: Option<&[u8; 32]>,
) -> Result<BookmarkView> {
    let existing = get_bookmark_raw(pool, user_id, id).await?;
    if existing.is_vault {
        let key = vault_key.ok_or(Error::VaultLocked)?;
        let mut payload = crypto::decrypt_payload(
            key,
            existing
                .vault_ciphertext
                .as_deref()
                .ok_or(Error::msg("missing ciphertext"))?,
            existing
                .vault_nonce
                .as_deref()
                .ok_or(Error::msg("missing nonce"))?,
        )?;
        if let Some(url) = req.url {
            payload.url = url;
        }
        if let Some(title) = req.title {
            payload.title = title;
        }
        if let Some(description) = req.description {
            payload.description = Some(description);
        }
        if let Some(notes) = req.notes {
            payload.notes = Some(notes);
        }
        if let Some(tags) = req.tags {
            payload.tags = tags;
        }
        let (ct, nonce) = crypto::encrypt_payload(key, &payload)?;
        let b = sqlx::query_as::<_, Bookmark>(
            r#"
            UPDATE bookmarks
            SET vault_ciphertext = $1, vault_nonce = $2, updated_at = now()
            WHERE id = $3 AND owner_user_id = $4
            RETURNING *
            "#,
        )
        .bind(ct)
        .bind(nonce)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if let Some(ids) = req.category_ids {
            set_bookmark_categories(pool, user_id, id, &ids).await?;
        } else if let Some(c) = req.category_id {
            set_bookmark_categories(pool, user_id, id, &c.into_iter().collect::<Vec<_>>()).await?;
        }
        let b = get_bookmark_raw(pool, user_id, id).await?;
        return bookmark_to_view(pool, b, vault_key)
            .await?
            .ok_or(Error::VaultLocked);
    }

    let url = req.url.unwrap_or(existing.url);
    let title = req.title.unwrap_or(existing.title);
    let description = req.description.or(existing.description);
    let notes = req.notes.or(existing.notes);
    let b = sqlx::query_as::<_, Bookmark>(
        r#"
        UPDATE bookmarks
        SET url=$1, title=$2, description=$3, notes=$4, updated_at=now()
        WHERE id=$5 AND owner_user_id=$6
        RETURNING *
        "#,
    )
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&notes)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if let Some(tags) = req.tags {
        set_bookmark_tags(pool, user_id, id, &tags).await?;
    }
    if let Some(ids) = req.category_ids {
        set_bookmark_categories(pool, user_id, id, &ids).await?;
    } else if let Some(c) = req.category_id {
        set_bookmark_categories(pool, user_id, id, &c.into_iter().collect::<Vec<_>>()).await?;
    }
    let b = get_bookmark_raw(pool, user_id, id).await?;
    bookmark_to_view(pool, b, None)
        .await?
        .ok_or(Error::NotFound)
}

pub async fn delete_bookmark(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let res = sqlx::query("DELETE FROM bookmarks WHERE id = $1 AND owner_user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn update_bookmark_metadata(
    pool: &PgPool,
    id: Uuid,
    title: Option<&str>,
    description: Option<&str>,
    favicon_url: Option<&str>,
    preview_image_url: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            favicon_url = COALESCE($4, favicon_url),
            preview_image_url = COALESCE($5, preview_image_url),
            updated_at = now()
        WHERE id = $1 AND is_vault = FALSE
        "#,
    )
    .bind(id)
    .bind(title)
    .bind(description)
    .bind(favicon_url)
    .bind(preview_image_url)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn set_bookmark_has_preview(pool: &PgPool, id: Uuid, has_preview: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET has_preview = $2, updated_at = now()
        WHERE id = $1 AND is_vault = FALSE
        "#,
    )
    .bind(id)
    .bind(has_preview)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Non-vault bookmarks that still need a local screenshot (owner_id, bookmark_id).
pub async fn list_bookmarks_needing_preview(pool: &PgPool) -> Result<Vec<(Uuid, Uuid)>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT owner_user_id, id FROM bookmarks
        WHERE is_vault = FALSE AND has_preview = FALSE AND url <> ''
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows)
}

pub async fn find_duplicate_urls(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT url FROM bookmarks
        WHERE owner_user_id = $1 AND is_vault = FALSE AND url <> ''
        GROUP BY url HAVING COUNT(*) > 1
        ORDER BY url
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|(u,)| u).collect())
}

pub async fn search_vault_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
    vault_key: &[u8; 32],
) -> Result<Vec<BookmarkView>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        "SELECT * FROM bookmarks WHERE owner_user_id = $1 AND is_vault = TRUE",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let q = query.to_ascii_lowercase();
    let mut out = Vec::new();
    for b in rows {
        if let Some(v) = bookmark_to_view(pool, b, Some(vault_key)).await? {
            let hay = format!(
                "{} {} {} {} {}",
                v.title,
                v.url,
                v.description.clone().unwrap_or_default(),
                v.notes.clone().unwrap_or_default(),
                v.tags.join(" ")
            )
            .to_ascii_lowercase();
            if hay.contains(&q) {
                out.push(v);
            }
        }
    }
    Ok(out)
}

pub async fn setup_vault(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<()> {
    if user_has_vault(pool, user_id).await? {
        return Err(Error::Conflict("vault already exists".into()));
    }
    let keys = crypto::setup_vault(passphrase)?;
    sqlx::query(
        r#"
        INSERT INTO vaults (user_id, kdf_salt, wrapped_key, wrapped_nonce)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(&keys.kdf_salt)
    .bind(&keys.wrapped_key)
    .bind(&keys.wrapped_nonce)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn get_vault(pool: &PgPool, user_id: Uuid) -> Result<Vault> {
    sqlx::query_as::<_, Vault>("SELECT * FROM vaults WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

pub async fn unlock_vault_key(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<[u8; 32]> {
    let vault = get_vault(pool, user_id).await?;
    crypto::unlock_vault(
        passphrase,
        &vault.kdf_salt,
        &vault.wrapped_key,
        &vault.wrapped_nonce,
    )
}

pub async fn create_import_job(
    pool: &PgPool,
    user_id: Uuid,
    source: &str,
) -> Result<ImportJob> {
    sqlx::query_as::<_, ImportJob>(
        r#"
        INSERT INTO import_jobs (owner_user_id, source, status)
        VALUES ($1, $2, 'running')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(source)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn finish_import_job(
    pool: &PgPool,
    id: Uuid,
    total: i32,
    imported: i32,
    failed: i32,
    error: Option<&str>,
) -> Result<ImportJob> {
    let status = if error.is_some() { "failed" } else { "done" };
    sqlx::query_as::<_, ImportJob>(
        r#"
        UPDATE import_jobs
        SET status=$2, total=$3, imported=$4, failed=$5, error=$6, updated_at=now()
        WHERE id=$1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(total)
    .bind(imported)
    .bind(failed)
    .bind(error)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn apply_parsed_import(
    pool: &PgPool,
    user_id: Uuid,
    parsed: linkdean_core::import_export::ParsedImport,
) -> Result<(i32, i32, i32)> {
    use linkdean_core::import_export::{leaf_name, path_key};

    let mut path_to_id: HashMap<String, Uuid> = HashMap::new();
    for (cat, path_entry) in parsed
        .categories
        .into_iter()
        .zip(parsed.category_paths.into_iter())
    {
        let (path, parent_path) = path_key(&path_entry);
        let parent_id = parent_path.and_then(|p| path_to_id.get(p).copied());
        let created = create_category(
            pool,
            user_id,
            CreateCategory {
                name: leaf_name(path).to_string(),
                parent_id,
                is_hidden: cat.is_hidden,
                is_vault: false,
            },
        )
        .await?;
        path_to_id.insert(path.to_string(), created.id);
    }

    let total = parsed.bookmarks.len() as i32;
    let mut imported = 0i32;
    let mut failed = 0i32;
    for (mut bm, cat_path) in parsed.bookmarks {
        bm.category_id = cat_path.and_then(|p| path_to_id.get(&p).copied());
        match create_bookmark(pool, user_id, bm, None).await {
            Ok(_) => imported += 1,
            Err(_) => failed += 1,
        }
    }
    Ok((total, imported, failed))
}

pub async fn export_data(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(
    Vec<linkdean_core::CategoryExport>,
    Vec<linkdean_core::BookmarkExport>,
)> {
    let cats = list_categories(pool, user_id, true, false).await?;
    let cat_exports = cats
        .into_iter()
        .map(|c| linkdean_core::CategoryExport {
            id: c.id,
            parent_id: c.parent_id,
            name: c.name,
            is_hidden: c.is_hidden,
            is_vault: c.is_vault,
            position: c.position,
        })
        .collect::<Vec<_>>();

    let bookmarks = list_bookmarks(pool, user_id, None, false, None).await?;
    let bm_exports = bookmarks
        .into_iter()
        .map(|b| linkdean_core::BookmarkExport {
            url: b.url,
            title: b.title,
            description: b.description,
            notes: b.notes,
            category_id: b.category_id,
            tags: b.tags,
            is_vault: false,
        })
        .collect();
    Ok((cat_exports, bm_exports))
}
