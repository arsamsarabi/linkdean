//! Experimental Streamable HTTP MCP at `/api/mcp` (nested as `/mcp` under `/api`).
//! Gated by the same `EXPERIMENTAL_AGENT_SKILLS` flag as the Agent Skill zip route.

mod server;

use crate::auth::AuthUser;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{FromRequestParts, Request, State};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use rmcp::transport::streamable_http_server::{
    session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use std::sync::Arc;
use tower::ServiceExt;

/// Mounted at `/mcp` inside the `/api` nest → final path `/api/mcp`.
pub fn router() -> Router<AppState> {
    // Explicit route (not nest+fallback): axum does not inherit a nested-only
    // fallback onto the exact `/mcp` path when merging into the API router.
    Router::new().route("/mcp", any(dispatch))
}

fn mcp_service(app_url: &str) -> StreamableHttpService<server::LinkdeanMcp, NeverSessionManager> {
    let mut hosts = vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
    if let Ok(url) = url::Url::parse(app_url) {
        if let Some(host) = url.host_str() {
            hosts.push(host.to_string());
            if let Some(port) = url.port() {
                hosts.push(format!("{host}:{port}"));
            } else if let Some(port) = url.port_or_known_default() {
                hosts.push(format!("{host}:{port}"));
            }
        }
    }
    StreamableHttpService::new(
        || Ok(server::LinkdeanMcp),
        Arc::new(NeverSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_allowed_hosts(hosts),
    )
}

async fn dispatch(State(state): State<AppState>, req: Request) -> Response {
    let (mut parts, body) = req.into_parts();
    let user = match AuthUser::from_request_parts(&mut parts, &state).await {
        Ok(AuthUser(user)) => user,
        Err(err) => return err.into_response(),
    };
    parts.extensions.insert(user);
    let req = Request::from_parts(parts, body);
    match mcp_service(&state.cfg.app_url).oneshot(req).await {
        Ok(res) => res.map(Body::new).into_response(),
        Err(infallible) => match infallible {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::routes::api_router;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use axum::Router;
    use linkdean_db as db;
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn state() -> Option<(AppState, std::path::PathBuf)> {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("skipping mcp http tests: DATABASE_URL unset");
            return None;
        };
        let dir = std::env::temp_dir().join(format!("ld-mcp-{}", Uuid::new_v4().simple()));
        for (k, v) in [
            ("DATABASE_URL", url.as_str()),
            ("SESSION_SECRET", "mcp-scaffold-session-secret-01"),
            ("ALLOW_DEFAULT_SECRETS", "true"),
            ("MEILI_URL", "http://127.0.0.1:9"),
            ("MEILI_MASTER_KEY", "mcp-scaffold-meili-key-000001"),
            ("APP_URL", "http://localhost:5199"),
            ("AVATAR_DIR", dir.join("avatars").to_str().unwrap()),
            ("PREVIEW_DIR", dir.join("previews").to_str().unwrap()),
            ("EXPERIMENTAL_AGENT_SKILLS", "false"),
        ] {
            // Edition 2021: setting variables is safe. No other test reads these.
            std::env::set_var(k, v);
        }
        let cfg = Config::from_env().expect("config");
        Some((AppState::new(cfg).await.expect("state"), dir))
    }

    fn app(state: AppState, agent_skills: bool) -> Router {
        Router::new()
            .nest("/api", api_router(false, agent_skills))
            .with_state(state)
    }

    async fn api_key(pool: &sqlx::PgPool) -> String {
        let email = format!("mcp-{}@mcp.test", Uuid::new_v4().simple());
        let u = db::create_user(pool, &email, Some("mcp-password"), None, None, None)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE users SET onboarding_completed = true, email_verified_at = now() WHERE id = $1",
        )
        .bind(u.id)
        .execute(pool)
        .await
        .unwrap();
        let u = db::find_user_by_id(pool, u.id).await.unwrap().unwrap();
        db::create_api_key(pool, &u, linkdean_core::CreateApiKey { name: "mcp".into() })
            .await
            .unwrap()
            .token
    }

    fn initialize_body() -> Body {
        Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "linkdean-mcp-test", "version": "0.0.1"}
                }
            })
            .to_string(),
        )
    }

    fn mcp_post(auth: Option<&str>) -> Request<Body> {
        let mut req = Request::builder()
            .method(Method::POST)
            .uri("/api/mcp")
            .header(header::HOST, "localhost")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");
        if let Some(token) = auth {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        req.body(initialize_body()).unwrap()
    }

    async fn json_status(app: Router, req: Request<Body>) -> (StatusCode, Value) {
        let res = app.oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn mcp_is_absent_when_experimental_flag_is_off() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let app = app(state, false);
        let (status, _) = json_status(app, mcp_post(None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn mcp_rejects_unauthenticated() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let app = app(state, true);
        let (status, body) = json_status(app, mcp_post(None)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "session_expired");
    }

    #[tokio::test]
    async fn mcp_initialize_ok_with_api_key() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let key = api_key(&state.pool).await;
        let app = app(state, true);
        let (status, body) = json_status(app, mcp_post(Some(&key))).await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        assert!(
            body["result"]["serverInfo"]["name"].is_string(),
            "missing serverInfo: {body}"
        );
        assert!(
            body["result"]["capabilities"]["tools"].is_object()
                || body["result"]["capabilities"]
                    .as_object()
                    .is_some_and(|c| c.contains_key("tools")),
            "tools capability missing: {body}"
        );
    }
}
