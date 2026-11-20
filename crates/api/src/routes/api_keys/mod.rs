use crate::auth::{AuthUser, SessionUser};
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::{CreateApiKey, CreateApiKeyResponse};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_keys, create_key))
        .routes(routes!(revoke_key))
}

#[utoipa::path(
    get,
    operation_id = "api_keys_list_keys",
    path = "/api-keys",
    tag = "api-keys",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_keys(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    db::require_api_keys_allowed(&state.pool, &user).await?;
    let keys = db::list_api_keys(&state.pool, user.id).await?;
    Ok(Json(json!({ "keys": keys })))
}

/// Login sessions only: a key that could mint keys would survive its own revocation.
#[utoipa::path(
    post,
    operation_id = "api_keys_create_key",
    path = "/api-keys",
    tag = "api-keys",
    responses(
        (status = 200, description = "OK", body = CreateApiKeyResponse),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn create_key(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(req): Json<CreateApiKey>,
) -> ApiResult<Json<CreateApiKeyResponse>> {
    let created = db::create_api_key(&state.pool, &user, req).await?;
    Ok(Json(created))
}

#[utoipa::path(
    delete,
    operation_id = "api_keys_revoke_key",
    path = "/api-keys/{id}",
    tag = "api-keys",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn revoke_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    // Revoke always allowed for own keys (even if gates later flip off)
    db::revoke_api_key(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod scope_http_tests {
    use super::*;
    use crate::config::Config;
    use crate::routes::api_router;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use axum::Router;
    use linkdean_core::{CreateApiKey, CreateBookmark, User, VaultKeyring};
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn state() -> Option<AppState> {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("skipping api key scope http tests: DATABASE_URL unset");
            return None;
        };
        let dir = std::env::temp_dir().join(format!("ld-scopes-{}", Uuid::new_v4().simple()));
        for (k, v) in [
            ("DATABASE_URL", url.as_str()),
            ("SESSION_SECRET", "api-key-scopes-session-secret-01"),
            ("ALLOW_DEFAULT_SECRETS", "true"),
            ("APP_URL", "http://localhost:5199"),
            ("AVATAR_DIR", dir.join("avatars").to_str().unwrap()),
            ("PREVIEW_DIR", dir.join("previews").to_str().unwrap()),
            ("ARCHIVE_DIR", dir.join("archives").to_str().unwrap()),
        ] {
            std::env::set_var(k, v);
        }
        let cfg = Config::from_env().expect("config");
        Some(AppState::new(cfg).await.expect("state"))
    }

    async fn verified(pool: &sqlx::PgPool) -> User {
        let email = format!("scopes-{}@test", Uuid::new_v4().simple());
        let u = db::create_user(pool, &email, Some("scopes-password"), None, None, None)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE users SET onboarding_completed = true, email_verified_at = now() WHERE id = $1",
        )
        .bind(u.id)
        .execute(pool)
        .await
        .unwrap();
        db::find_user_by_id(pool, u.id).await.unwrap().unwrap()
    }

    #[tokio::test]
    async fn library_read_key_cannot_create_bookmark() {
        let Some(state) = state().await else {
            return;
        };
        let user = verified(&state.pool).await;
        let key = db::create_api_key(
            &state.pool,
            &user,
            CreateApiKey {
                name: "ro".into(),
                scopes: Some(vec!["library:read".into()]),
            },
        )
        .await
        .unwrap()
        .token;

        let app: Router = Router::new()
            .nest("/api", api_router(false, false))
            .with_state(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/bookmarks")
                    .header(header::AUTHORIZATION, format!("Bearer {key}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "url": format!("https://scope.example/{}", Uuid::new_v4()),
                            "title": "Nope"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn library_read_key_can_list_bookmarks() {
        let Some(state) = state().await else {
            return;
        };
        let user = verified(&state.pool).await;
        db::create_bookmark(
            &state.pool,
            user.id,
            CreateBookmark {
                url: format!("https://scope-list.example/{}", Uuid::new_v4()),
                title: Some("Listed".into()),
                ..Default::default()
            },
            &VaultKeyring::default(),
        )
        .await
        .unwrap();
        let key = db::create_api_key(
            &state.pool,
            &user,
            CreateApiKey {
                name: "ro-list".into(),
                scopes: Some(vec!["library:read".into()]),
            },
        )
        .await
        .unwrap()
        .token;

        let app: Router = Router::new()
            .nest("/api", api_router(false, false))
            .with_state(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/bookmarks")
                    .header(header::AUTHORIZATION, format!("Bearer {key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
