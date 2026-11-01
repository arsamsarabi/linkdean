use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use linkdean_core::avatar::{self, process_avatar};
use linkdean_core::{ChangePassword, Error, UpdateProfile, UpdateSettings};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/me", patch(update_me))
        .route("/users/me/password", post(change_password))
        .route(
            "/users/me/settings",
            get(get_settings).patch(patch_settings),
        )
        .route(
            "/users/me/avatar",
            get(get_avatar).post(upload_avatar).delete(delete_avatar),
        )
        .route("/users/me/followers", get(my_followers))
        .route("/users/me/following", get(my_following))
        .route(
            "/users/{id}/follow",
            post(follow_user).delete(unfollow_user),
        )
        .route("/users/{id}/follow-status", get(follow_status))
}

async fn update_me(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<UpdateProfile>,
) -> ApiResult<Json<Value>> {
    let user = db::update_profile(&state.pool, user.id, &body).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

async fn change_password(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ChangePassword>,
) -> ApiResult<Json<Value>> {
    db::change_password(
        &state.pool,
        user.id,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn get_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    Ok(Json(json!({ "settings": settings })))
}

async fn patch_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<UpdateSettings>,
) -> ApiResult<Json<Value>> {
    let settings = db::update_user_settings(&state.pool, user.id, &body).await?;
    Ok(Json(json!({ "settings": settings })))
}

async fn upload_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> ApiResult<Json<Value>> {
    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        let ct = field.content_type().map(|s| s.to_string());
        if name != "avatar" && name != "file" {
            continue;
        }
        if !avatar::accept_content_type(ct.as_deref()) {
            return Err(Error::BadRequest("unsupported content type".into()).into());
        }
        let data = field
            .bytes()
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        bytes = Some(data.to_vec());
        break;
    }
    let raw = bytes.ok_or_else(|| Error::BadRequest("missing avatar file field".into()))?;
    let webp = tokio::task::spawn_blocking(move || process_avatar(&raw))
        .await
        .map_err(|e| Error::msg(e.to_string()))??;
    let path = state.avatar_path(user.id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    }
    tokio::fs::write(&path, &webp)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let user = db::set_has_avatar(&state.pool, user.id, true).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

async fn delete_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let path = state.avatar_path(user.id);
    if let Err(e) = tokio::fs::remove_file(&path).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(?path, error = %e, "avatar delete failed");
        }
    }
    let user = db::set_has_avatar(&state.pool, user.id, false).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

async fn get_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<impl IntoResponse> {
    if !user.has_avatar {
        return Err(Error::NotFound.into());
    }
    let path = state.avatar_path(user.id);
    let bytes = tokio::fs::read(&path).await.map_err(|_| Error::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    Ok((StatusCode::OK, headers, bytes))
}

async fn my_followers(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_followers(&state.pool, user.id).await?;
    Ok(Json(json!({ "users": users })))
}

async fn my_following(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_following(&state.pool, user.id).await?;
    Ok(Json(json!({ "users": users })))
}

async fn follow_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::follow_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "follow": relation })))
}

async fn unfollow_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::unfollow_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "follow": relation })))
}

async fn follow_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::follow_relation(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "follow": relation })))
}
