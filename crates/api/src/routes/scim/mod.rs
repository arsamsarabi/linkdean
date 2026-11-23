//! SCIM 2.0 Users for organization provisioning (Phase D3).
//!
//! Authenticated with an `ldscim_` bearer token from Organization settings → SSO.

use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use linkdean_core::{Error, User};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(service_provider_config))
        .routes(routes!(list_users))
        .routes(routes!(create_user))
        .routes(routes!(get_user))
        .routes(routes!(replace_user))
        .routes(routes!(patch_user))
        .routes(routes!(delete_user))
}

async fn org_from_bearer(state: &AppState, headers: &HeaderMap) -> Result<Uuid, Error> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .unwrap_or("")
        .trim();
    if !db::is_scim_token(token) {
        return Err(Error::Unauthorized);
    }
    let (_token_id, organization_id) = db::authenticate_scim_token(&state.pool, token).await?;
    Ok(organization_id)
}

fn scim_error(status: StatusCode, detail: &str) -> Response {
    let body = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
        "detail": detail,
        "status": status.as_u16().to_string(),
    });
    (status, Json(body)).into_response()
}

fn to_scim_user(user: &User, location_base: &str, active: bool) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": user.id,
        "userName": user.email,
        "name": {
            "formatted": user.display_name.clone().unwrap_or_else(|| user.email.clone()),
        },
        "emails": [{ "value": user.email, "primary": true, "type": "work" }],
        "active": active,
        "meta": {
            "resourceType": "User",
            "created": user.created_at,
            "lastModified": user.updated_at,
            "location": format!("{location_base}/{}", user.id),
        }
    })
}

fn pick_email(body: &Value) -> Option<String> {
    if let Some(u) = body.get("userName").and_then(|v| v.as_str()) {
        let t = u.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    body.get("emails")
        .and_then(|e| e.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|e| e.get("primary").and_then(|p| p.as_bool()) == Some(true))
                .or_else(|| arr.first())
        })
        .and_then(|e| e.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn pick_display_name(body: &Value) -> Option<String> {
    body.get("name")
        .and_then(|n| n.get("formatted").or_else(|| n.get("givenName")))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            body.get("displayName")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

fn pick_active(body: &Value) -> bool {
    body.get("active").and_then(|v| v.as_bool()).unwrap_or(true)
}

fn users_location(state: &AppState) -> String {
    format!(
        "{}/api/scim/v2/Users",
        state.cfg.app_url.trim_end_matches('/')
    )
}

#[utoipa::path(
    get,
    operation_id = "scim_service_provider_config",
    path = "/scim/v2/ServiceProviderConfig",
    tag = "scim",
    responses((status = 200, description = "OK")),
)]
async fn service_provider_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    org_from_bearer(&state, &headers).await?;
    Ok(Json(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
        "patch": { "supported": true },
        "bulk": { "supported": false, "maxOperations": 0, "maxPayloadSize": 0 },
        "filter": { "supported": true, "maxResults": 200 },
        "changePassword": { "supported": false },
        "sort": { "supported": false },
        "etag": { "supported": false },
        "authenticationSchemes": [{
            "type": "oauthbearertoken",
            "name": "OAuth Bearer Token",
            "description": "Linkdean organization SCIM token (ldscim_…)",
            "specUri": "https://www.rfc-editor.org/rfc/rfc6750.html",
            "primary": true
        }]
    }))
    .into_response())
}

#[derive(Deserialize)]
struct ListQuery {
    filter: Option<String>,
    #[serde(default = "one", rename = "startIndex")]
    start_index: i64,
    #[serde(default = "hundred")]
    count: i64,
}

fn one() -> i64 {
    1
}
fn hundred() -> i64 {
    100
}

fn parse_username_eq(filter: &str) -> Option<String> {
    // userName eq "jane@example.com"
    let f = filter.trim();
    let lower = f.to_ascii_lowercase();
    let rest = lower.strip_prefix("username eq ")?;
    let orig_rest = &f[f.len() - rest.len()..];
    let quoted = orig_rest.trim().trim_matches('"').trim_matches('\'').trim();
    if quoted.is_empty() {
        None
    } else {
        Some(quoted.to_string())
    }
}

#[utoipa::path(
    get,
    operation_id = "scim_list_users",
    path = "/scim/v2/Users",
    tag = "scim",
    responses((status = 200, description = "OK")),
)]
async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> ApiResult<Response> {
    let org_id = org_from_bearer(&state, &headers).await?;
    let filter_email = q.filter.as_deref().and_then(parse_username_eq);
    let users = db::scim_list_org_members(&state.pool, org_id, filter_email.as_deref()).await?;
    let base = users_location(&state);
    let resources: Vec<_> = users.iter().map(|u| to_scim_user(u, &base, true)).collect();
    let total = resources.len() as i64;
    let start = q.start_index.max(1);
    let count = q.count.clamp(1, 200);
    let skip = (start - 1) as usize;
    let page: Vec<_> = resources
        .into_iter()
        .skip(skip)
        .take(count as usize)
        .collect();
    Ok(Json(json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": total,
        "startIndex": start,
        "itemsPerPage": page.len(),
        "Resources": page,
    }))
    .into_response())
}

#[utoipa::path(
    post,
    operation_id = "scim_create_user",
    path = "/scim/v2/Users",
    tag = "scim",
    request_body = serde_json::Value,
    responses((status = 201, description = "Created")),
)]
async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<Response> {
    let org_id = org_from_bearer(&state, &headers).await?;
    let Some(email) = pick_email(&body) else {
        return Ok(scim_error(
            StatusCode::BAD_REQUEST,
            "userName or emails required",
        ));
    };
    if let Some(existing) = db::find_user_by_email(&state.pool, &email).await? {
        if db::membership_role(&state.pool, existing.id, org_id)
            .await?
            .is_some()
        {
            return Ok(scim_error(StatusCode::CONFLICT, "User already exists"));
        }
    }
    let active = pick_active(&body);
    let user = match db::scim_provision_user(
        &state.pool,
        org_id,
        &email,
        pick_display_name(&body).as_deref(),
        active,
    )
    .await
    {
        Ok(u) => u,
        Err(Error::BadRequest(m)) => return Ok(scim_error(StatusCode::BAD_REQUEST, &m)),
        Err(Error::Conflict(m)) => return Ok(scim_error(StatusCode::CONFLICT, &m)),
        Err(e) => return Err(e.into()),
    };
    let base = users_location(&state);
    let mut res = (
        StatusCode::CREATED,
        Json(to_scim_user(&user, &base, active)),
    )
        .into_response();
    if let Ok(loc) = format!("{base}/{}", user.id).parse() {
        res.headers_mut().insert(header::LOCATION, loc);
    }
    Ok(res)
}

#[utoipa::path(
    get,
    operation_id = "scim_get_user",
    path = "/scim/v2/Users/{id}",
    tag = "scim",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "OK")),
)]
async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let org_id = org_from_bearer(&state, &headers).await?;
    match db::scim_get_org_member(&state.pool, org_id, id).await {
        Ok(user) => Ok(Json(to_scim_user(&user, &users_location(&state), true)).into_response()),
        Err(Error::NotFound) => Ok(scim_error(StatusCode::NOT_FOUND, "User not found")),
        Err(e) => Err(e.into()),
    }
}

#[utoipa::path(
    put,
    operation_id = "scim_replace_user",
    path = "/scim/v2/Users/{id}",
    tag = "scim",
    params(("id" = Uuid, Path)),
    request_body = serde_json::Value,
    responses((status = 200, description = "OK")),
)]
async fn replace_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> ApiResult<Response> {
    let org_id = org_from_bearer(&state, &headers).await?;
    let _ = db::scim_get_org_member(&state.pool, org_id, id)
        .await
        .map_err(|e| match e {
            Error::NotFound => Error::NotFound,
            other => other,
        })?;
    let Some(email) = pick_email(&body) else {
        return Ok(scim_error(
            StatusCode::BAD_REQUEST,
            "userName or emails required",
        ));
    };
    let active = pick_active(&body);
    let user = match db::scim_provision_user(
        &state.pool,
        org_id,
        &email,
        pick_display_name(&body).as_deref(),
        active,
    )
    .await
    {
        Ok(u) => u,
        Err(Error::BadRequest(m)) => return Ok(scim_error(StatusCode::BAD_REQUEST, &m)),
        Err(e) => return Err(e.into()),
    };
    if user.id != id && active {
        // Email pointed at a different account — still ok if we provisioned by email.
    }
    Ok(Json(to_scim_user(&user, &users_location(&state), active)).into_response())
}

#[utoipa::path(
    patch,
    operation_id = "scim_patch_user",
    path = "/scim/v2/Users/{id}",
    tag = "scim",
    params(("id" = Uuid, Path)),
    request_body = serde_json::Value,
    responses((status = 200, description = "OK")),
)]
async fn patch_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> ApiResult<Response> {
    let org_id = org_from_bearer(&state, &headers).await?;
    let user = match db::scim_get_org_member(&state.pool, org_id, id).await {
        Ok(u) => u,
        Err(Error::NotFound) => return Ok(scim_error(StatusCode::NOT_FOUND, "User not found")),
        Err(e) => return Err(e.into()),
    };
    let mut active = true;
    let ops = body
        .get("Operations")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for op in ops {
        let kind = op
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if kind != "replace" && kind != "add" {
            continue;
        }
        if op.get("path").and_then(|v| v.as_str()) == Some("active") {
            active = match op.get("value") {
                Some(Value::Bool(b)) => *b,
                Some(Value::String(s)) => s.eq_ignore_ascii_case("true"),
                _ => active,
            };
        } else if let Some(Value::Object(map)) = op.get("value") {
            if let Some(v) = map.get("active") {
                active = v.as_bool().unwrap_or(active);
            }
        }
    }
    if active {
        let _ = db::scim_provision_user(
            &state.pool,
            org_id,
            &user.email,
            user.display_name.as_deref(),
            true,
        )
        .await?;
    } else {
        db::scim_deprovision_user(&state.pool, org_id, id).await?;
    }
    let fresh = db::find_user_by_id(&state.pool, id)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(Json(to_scim_user(&fresh, &users_location(&state), active)).into_response())
}

#[utoipa::path(
    delete,
    operation_id = "scim_delete_user",
    path = "/scim/v2/Users/{id}",
    tag = "scim",
    params(("id" = Uuid, Path)),
    responses((status = 204, description = "No content")),
)]
async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let org_id = org_from_bearer(&state, &headers).await?;
    match db::scim_get_org_member(&state.pool, org_id, id).await {
        Ok(_) => {}
        Err(Error::NotFound) => return Ok(scim_error(StatusCode::NOT_FOUND, "User not found")),
        Err(e) => return Err(e.into()),
    }
    match db::scim_deprovision_user(&state.pool, org_id, id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(Error::BadRequest(m)) => Ok(scim_error(StatusCode::BAD_REQUEST, &m)),
        Err(e) => Err(e.into()),
    }
}
