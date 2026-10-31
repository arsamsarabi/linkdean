use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::SearchHit;
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

fn clamp_limit(limit: usize) -> usize {
    limit.clamp(1, 100)
}

async fn search(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Value>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let mut hits = Vec::new();
    let mut vault_prompt = false;
    let limit = clamp_limit(q.limit);

    if !q.q.trim().is_empty() {
        // Owned + shared category ids so Meili can surface shared (and invitee-created) docs.
        let readable = db::list_categories(&state.pool, user.id, true, false, false).await?;
        let readable_ids: Vec<Uuid> = readable.into_iter().map(|c| c.id).collect();

        match state
            .search
            .search(user.id, &readable_ids, &q.q, limit)
            .await
        {
            Ok(docs) => {
                for d in docs {
                    let id = Uuid::parse_str(&d.id).unwrap_or_else(|_| Uuid::nil());
                    let owner_user_id =
                        Uuid::parse_str(&d.owner_user_id).unwrap_or(user.id);
                    let can_write = db::bookmark_can_write(&state.pool, user.id, id)
                        .await
                        .unwrap_or(false);
                    hits.push(SearchHit {
                        id,
                        owner_user_id,
                        title: d.title,
                        url: d.url,
                        description: if d.description.is_empty() {
                            None
                        } else {
                            Some(d.description)
                        },
                        favicon_url: d.favicon_url,
                        preview_image_url: d.preview_image_url,
                        has_preview: d.has_preview,
                        tags: d.tags,
                        category_id: d
                            .category_id
                            .as_deref()
                            .and_then(|s| Uuid::parse_str(s).ok()),
                        is_vault: false,
                        can_write,
                    });
                }
            }
            Err(e) => tracing::warn!("meili search failed: {e}"),
        }

        let mode = settings.vault_search_mode.as_str();
        let unlocked = state.vault_key(user.id).await;
        match mode {
            "never" => {}
            "when_unlocked" => {
                if let Some(key) = unlocked {
                    let vault_hits =
                        db::search_vault_bookmarks(&state.pool, user.id, &q.q, &key).await?;
                    for v in vault_hits {
                        hits.push(SearchHit {
                            id: v.id,
                            owner_user_id: v.owner_user_id,
                            title: v.title,
                            url: v.url,
                            description: v.description,
                            favicon_url: v.favicon_url,
                            preview_image_url: v.preview_image_url,
                            has_preview: v.has_preview,
                            tags: v.tags,
                            category_id: v.category_id,
                            is_vault: true,
                            can_write: v.can_write,
                        });
                    }
                }
            }
            "always" => {
                if let Some(key) = unlocked {
                    let vault_hits =
                        db::search_vault_bookmarks(&state.pool, user.id, &q.q, &key).await?;
                    for v in vault_hits {
                        hits.push(SearchHit {
                            id: v.id,
                            owner_user_id: v.owner_user_id,
                            title: v.title,
                            url: v.url,
                            description: v.description,
                            favicon_url: v.favicon_url,
                            preview_image_url: v.preview_image_url,
                            has_preview: v.has_preview,
                            tags: v.tags,
                            category_id: v.category_id,
                            is_vault: true,
                            can_write: v.can_write,
                        });
                    }
                } else {
                    vault_prompt = true;
                }
            }
            _ => {}
        }
    }

    Ok(Json(json!({
        "hits": hits,
        "vault_prompt": vault_prompt,
    })))
}
