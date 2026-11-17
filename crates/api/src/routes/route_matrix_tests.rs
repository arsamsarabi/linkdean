//! Every documented route, called the ways a client gets wrong: no credentials, the wrong kind
//! of credentials, a non-admin on an admin route, an id that does not exist, a broken JSON
//! body. The route list comes from the OpenAPI document, so a new route is covered as soon as
//! it is documented, and every route is documented because the routers are built from it.
//!
//! Needs Postgres: set `DATABASE_URL` (the throwaway test database). Skips otherwise.

use super::{api_router, openapi};
use crate::config::Config;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use linkdean_db as db;
use serde_json::Value;
use std::net::SocketAddr;
use tower::ServiceExt;
use uuid::Uuid;

struct Op {
    method: Method,
    path: String,
    /// Security requirement names; empty means public.
    security: Vec<String>,
    /// `security: [{}]` lists anonymous access as allowed.
    optional_auth: bool,
    json_body: bool,
}

fn operations() -> Vec<Op> {
    let spec = serde_json::to_value(openapi()).unwrap();
    let mut out = Vec::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for (method, op) in item.as_object().unwrap() {
            let reqs = op["security"].as_array().cloned().unwrap_or_default();
            out.push(Op {
                method: method.to_uppercase().parse().unwrap(),
                path: path.clone(),
                security: reqs
                    .iter()
                    .flat_map(|r| r.as_object().unwrap().keys().cloned())
                    .collect(),
                optional_auth: reqs.iter().any(|r| r.as_object().unwrap().is_empty()),
                json_body: op["requestBody"]["content"]
                    .get("application/json")
                    .is_some(),
            });
        }
    }
    out
}

/// A concrete path: random ids, valid names for the enum-like segments.
fn concrete(path: &str) -> String {
    path.split('/')
        .map(|seg| match seg {
            "{subject_type}" => "bookmark".to_string(),
            "{kind}" if path.starts_with("/trash") => "bookmark".to_string(),
            "{kind}" => "share_invite".to_string(),
            "{slug}" => format!("missing-{}", Uuid::new_v4().simple()),
            s if s.starts_with('{') => Uuid::new_v4().to_string(),
            s => s.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

async fn state() -> Option<(AppState, std::path::PathBuf)> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping route matrix: DATABASE_URL unset");
        return None;
    };
    let dir = std::env::temp_dir().join(format!("ld-route-matrix-{}", Uuid::new_v4().simple()));
    // Edition 2021: setting variables is safe. No other test reads these.
    for (k, v) in [
        ("DATABASE_URL", url.as_str()),
        ("SESSION_SECRET", "route-matrix-session-secret-01"),
        ("ALLOW_DEFAULT_SECRETS", "true"),
        ("MEILI_URL", "http://127.0.0.1:9"),
        ("MEILI_MASTER_KEY", "route-matrix-meili-key-0001"),
        ("APP_URL", "http://localhost:5199"),
        ("AVATAR_DIR", dir.join("avatars").to_str().unwrap()),
        ("PREVIEW_DIR", dir.join("previews").to_str().unwrap()),
    ] {
        std::env::set_var(k, v);
    }
    let cfg = Config::from_env().expect("config");
    Some((AppState::new(cfg).await.expect("state"), dir))
}

fn app(state: AppState) -> Router {
    api_router(false, true)
        .layer(axum::middleware::from_fn(
            crate::error::normalize_error_body,
        ))
        .layer(axum::middleware::from_fn(
            crate::http_cache::caching_headers,
        ))
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))))
        .with_state(state)
}

#[derive(Clone, Copy)]
enum Cred<'a> {
    None,
    Bearer(&'a str),
}

async fn call(app: &Router, op: &Op, cred: Cred<'_>, body: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(op.method.clone())
        .uri(concrete(&op.path))
        .header("x-linkdean-client", "web");
    if let Cred::Bearer(t) = cred {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let is_json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|v| v.to_str().unwrap_or("").starts_with("application/json"));
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 22)
        .await
        .unwrap();
    let json = if is_json {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    (status, json)
}

/// An error answer has the shared shape.
fn assert_error_body(what: &str, status: StatusCode, body: &Value) {
    assert!(
        body["code"].is_string() && body["message"].is_string() && body["error"] == body["message"],
        "{what}: {status} without the error body: {body}"
    );
}

async fn user(pool: &sqlx::PgPool, admin: bool) -> (String, String) {
    let email = format!("route-matrix-{}@matrix.test", Uuid::new_v4().simple());
    let u = db::create_user(
        pool,
        &email,
        Some("route-matrix-password"),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE users SET is_admin = $2, onboarding_completed = true, email_verified_at = now() \
         WHERE id = $1",
    )
    .bind(u.id)
    .bind(admin)
    .execute(pool)
    .await
    .unwrap();
    let u = db::find_user_by_id(pool, u.id).await.unwrap().unwrap();
    let session = db::create_session(pool, u.id, 1).await.unwrap();
    let key = db::create_api_key(
        pool,
        &u,
        linkdean_core::CreateApiKey {
            name: "matrix".into(),
        },
    )
    .await
    .unwrap()
    .token;
    (session, key)
}

#[tokio::test]
async fn every_route_answers_auth_and_input_errors_in_the_shared_shape() {
    let Some((state, dir)) = state().await else {
        return;
    };
    let pool = state.pool.clone();
    let app = app(state);
    let (member_session, member_key) = user(&pool, false).await;
    let (_admin_session, _) = user(&pool, true).await;
    let ops = operations();
    assert!(ops.len() >= 190);
    let mut failures = Vec::new();
    let mut check = |cond: bool, msg: String| {
        if !cond {
            failures.push(msg);
        }
    };

    for op in &ops {
        let what = format!("{} {}", op.method, op.path);
        let secured = !op.security.is_empty() && !op.optional_auth;

        // 1. No credentials: secured routes refuse with 401 in the shared shape.
        let (s, b) = call(&app, op, Cred::None, None).await;
        // `/ready` reports 503 by design here: the test has no Meilisearch.
        check(
            !s.is_server_error() || op.path == "/ready",
            format!("{what} anonymous: {s} {b}"),
        );
        if secured {
            check(
                s == StatusCode::UNAUTHORIZED && b["code"] == "session_expired",
                format!("{what} anonymous: want 401 session_expired, got {s} {b}"),
            );
        }
        if s.is_client_error() {
            let ok = b["code"].is_string() && b["message"].is_string();
            check(
                ok,
                format!("{what} anonymous: {s} not in the error shape: {b}"),
            );
        }

        // 2. An API key where only a sign-in session may act.
        let session_only = op.security.iter().any(|s| s == "session_token")
            && !op.security.iter().any(|s| s == "api_key");
        if session_only {
            let (s, b) = call(&app, op, Cred::Bearer(&member_key), None).await;
            check(
                s == StatusCode::UNAUTHORIZED || s == StatusCode::FORBIDDEN,
                format!("{what} with an API key: want 401/403, got {s} {b}"),
            );
            if s.is_client_error() {
                assert_error_body(&what, s, &b);
            }
        }

        // 3. A member on an admin route.
        if op.path.starts_with("/admin") {
            let (s, b) = call(&app, op, Cred::Bearer(&member_session), None).await;
            check(
                s == StatusCode::FORBIDDEN && b["code"] == "forbidden",
                format!("{what} as a member: want 403 forbidden, got {s} {b}"),
            );
        }

        if !secured || op.path.starts_with("/admin") {
            continue;
        }

        // 4. A broken JSON body.
        if op.json_body {
            let (s, b) = call(&app, op, Cred::Bearer(&member_session), Some("{")).await;
            check(
                s.is_client_error() && !s.is_server_error(),
                format!("{what} broken body: {s} {b}"),
            );
            if s.is_client_error() {
                check(
                    b["code"].is_string() && b["message"].is_string(),
                    format!("{what} broken body: {s} not in the error shape: {b}"),
                );
            }
        }

        // 5. An id nobody has. The relationship reads answer "not following / not blocked /
        // not muted" for any id, so they never reveal whether a hidden account exists.
        let relationship = op.path.ends_with("-status");
        if op.method == Method::GET && op.path.contains('{') && !relationship {
            let (s, b) = call(&app, op, Cred::Bearer(&member_session), None).await;
            check(
                s == StatusCode::NOT_FOUND || s == StatusCode::FORBIDDEN,
                format!("{what} unknown id: want 404/403, got {s} {b}"),
            );
            if s.is_client_error() {
                check(
                    b["code"].is_string() && b["message"].is_string(),
                    format!("{what} unknown id: {s} not in the error shape: {b}"),
                );
            }
        }
    }

    sqlx::query("DELETE FROM users WHERE email LIKE 'route-matrix-%@matrix.test'")
        .execute(&pool)
        .await
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        failures.is_empty(),
        "{} route checks failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
