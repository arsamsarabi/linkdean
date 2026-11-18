use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Query, State};
use axum::Json;
use linkdean_core::SearchHit;
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(search))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
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

#[utoipa::path(
    get,
    operation_id = "search_search",
    path = "/search",
    tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn search(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Value>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let mut hits = Vec::new();
    let mut vault_prompt = false;
    let limit = clamp_limit(q.limit);

    if !q.q.trim().is_empty() {
        // Owned + shared category ids so Meili can surface shared (and invitee-created) docs.
        let readable = db::list_categories(&state.pool, user.id, true, &[], false).await?;
        let readable_ids: Vec<Uuid> = readable.into_iter().map(|c| c.id).collect();

        match state
            .search
            .search(user.id, &readable_ids, &q.q, limit)
            .await
        {
            Ok(docs) => {
                // The index is cleaned up after the database commits, and that can fail. Serve
                // only hits Postgres still has as live, non-vault and readable, and drop the
                // stale documents so they stop coming back.
                let ids: Vec<Uuid> = docs
                    .iter()
                    .filter_map(|d| Uuid::parse_str(&d.id).ok())
                    .collect();
                let live = db::live_plain_bookmark_ids(&state.pool, &ids).await?;
                for stale in ids.iter().filter(|id| !live.contains(id)) {
                    if let Err(e) = state.search.delete_bookmark(*stale).await {
                        tracing::warn!(bookmark_id = %stale, error = %e, "drop stale search document failed");
                    }
                }
                // Categories and tags come from Postgres so a card never shows Unsorted after
                // the library already moved the bookmark. Meili still ranks the hits.
                let views = db::plain_bookmark_views_by_ids(&state.pool, user.id, &ids).await?;
                for d in docs {
                    let Ok(id) = Uuid::parse_str(&d.id) else {
                        continue;
                    };
                    if !live.contains(&id)
                        || !db::bookmark_can_read(&state.pool, user.id, id)
                            .await
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    let Some(view) = views.get(&id) else {
                        continue;
                    };
                    // Keep Meili in sync when its denormalized fields lag a category/tag write.
                    if view.tags != d.tags
                        || view.category_id.map(|c| c.to_string()) != d.category_id
                        || view
                            .categories
                            .iter()
                            .map(|c| c.id.to_string())
                            .collect::<Vec<_>>()
                            != d.category_ids
                    {
                        if let Err(e) = crate::search_index::upsert(&state, view).await {
                            tracing::warn!(bookmark_id = %id, error = %e, "meili catch-up upsert failed");
                        }
                    }
                    let can_write = db::bookmark_can_write(&state.pool, user.id, id)
                        .await
                        .unwrap_or(false);
                    hits.push(SearchHit {
                        id,
                        owner_user_id: view.owner_user_id,
                        title: view.title.clone(),
                        url: view.url.clone(),
                        description: view.description.clone(),
                        favicon_url: view.favicon_url.clone(),
                        preview_image_url: view.preview_image_url.clone(),
                        has_preview: view.has_preview,
                        tags: view.tags.clone(),
                        category_id: view.category_id,
                        categories: view.categories.clone(),
                        is_vault: false,
                        can_write,
                    });
                }
            }
            Err(e) => tracing::warn!("meili search failed: {e}"),
        }

        let mode = if settings.show_vault_in_library {
            settings.vault_search_mode.as_str()
        } else {
            "never"
        };
        let keyring = state.vault_keyring(&vault_session, user.id).await;
        match mode {
            "never" => {}
            "when_unlocked" => {
                if !keyring.is_empty() {
                    let vault_hits =
                        db::search_vault_bookmarks(&state.pool, user.id, &q.q, &keyring).await?;
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
                            categories: v.categories,
                            is_vault: true,
                            can_write: v.can_write,
                        });
                    }
                }
            }
            "always" => {
                if !keyring.is_empty() {
                    let vault_hits =
                        db::search_vault_bookmarks(&state.pool, user.id, &q.q, &keyring).await?;
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
                            categories: v.categories,
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
