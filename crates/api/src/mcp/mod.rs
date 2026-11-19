//! Experimental Streamable HTTP MCP at `/api/mcp` (nested as `/mcp` under `/api`).
//! Gated by the same `EXPERIMENTAL_AGENT_SKILLS` flag as the Agent Skill zip route.

mod dto;
mod server;
mod tools;

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
        || Ok(server::LinkdeanMcp::new()),
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
    parts.extensions.insert(state.clone());
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
    use linkdean_core::{
        CreateApiKey, CreateBookmark, CreateCategory, CreateCategoryShare, ShareAccess, User,
        VaultKeyring,
    };
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
        // Real local Meili so search_bookmarks(q) can index + hit; ALLOW_DEFAULT_SECRETS
        // permits the published meili key used by the dev stack.
        for (k, v) in [
            ("DATABASE_URL", url.as_str()),
            ("SESSION_SECRET", "mcp-scaffold-session-secret-01"),
            ("ALLOW_DEFAULT_SECRETS", "true"),
            ("MEILI_URL", "http://127.0.0.1:7700"),
            ("MEILI_MASTER_KEY", "dev_master_key_change_me"),
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

    async fn verified_user(pool: &sqlx::PgPool, prefix: &str) -> User {
        let email = format!("{prefix}-{}@mcp.test", Uuid::new_v4().simple());
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
        db::find_user_by_id(pool, u.id).await.unwrap().unwrap()
    }

    async fn api_key_for(pool: &sqlx::PgPool, user: &User) -> String {
        db::create_api_key(pool, user, CreateApiKey { name: "mcp".into() })
            .await
            .unwrap()
            .token
    }

    async fn api_key(pool: &sqlx::PgPool) -> String {
        let u = verified_user(pool, "mcp").await;
        api_key_for(pool, &u).await
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

    fn tools_call(auth: &str, name: &str, arguments: Value) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/api/mcp")
            .header(header::HOST, "localhost")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-03-26")
            .header(header::AUTHORIZATION, format!("Bearer {auth}"))
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": { "name": name, "arguments": arguments }
                })
                .to_string(),
            ))
            .unwrap()
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

    fn tool_payload(body: &Value) -> Value {
        if let Some(sc) = body.pointer("/result/structuredContent") {
            return sc.clone();
        }
        if let Some(text) = body
            .pointer("/result/content/0/text")
            .and_then(|t| t.as_str())
        {
            return serde_json::from_str(text).unwrap_or(Value::Null);
        }
        Value::Null
    }

    fn tool_error_message(body: &Value) -> String {
        body.pointer("/error/message")
            .and_then(|m| m.as_str())
            .or_else(|| {
                body.pointer("/result/content/0/text")
                    .and_then(|t| t.as_str())
            })
            .unwrap_or("")
            .to_string()
    }

    async fn category(pool: &sqlx::PgPool, owner: &User, name: &str, parent: Option<Uuid>) -> Uuid {
        db::create_category(
            pool,
            owner.id,
            CreateCategory {
                name: name.into(),
                parent_id: parent,
                is_hidden: false,
                vault_id: None,
                icon: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn bookmark(
        pool: &sqlx::PgPool,
        owner: &User,
        title: &str,
        folder: Option<Uuid>,
        tags: &[&str],
        notes: Option<&str>,
    ) -> linkdean_core::BookmarkView {
        db::create_bookmark(
            pool,
            owner.id,
            CreateBookmark {
                url: format!("https://mcp-tools.example/{}", Uuid::new_v4()),
                title: Some(title.into()),
                description: Some(format!("About {title}")),
                notes: notes.map(str::to_string),
                category_ids: folder.into_iter().collect(),
                tags: tags.iter().map(|t| t.to_string()).collect(),
                ..Default::default()
            },
            &VaultKeyring::default(),
        )
        .await
        .unwrap()
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

    #[tokio::test]
    async fn search_with_q_returns_hits() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let owner = verified_user(&state.pool, "search-q").await;
        let key = api_key_for(&state.pool, &owner).await;
        let needle = format!("mcpq-{}", Uuid::new_v4().simple());
        let view = bookmark(
            &state.pool,
            &owner,
            &format!("Title {needle}"),
            None,
            &[],
            Some("secret note"),
        )
        .await;
        state
            .search
            .ensure_index()
            .await
            .expect("Meili must be reachable at MEILI_URL for search_with_q_returns_hits");
        state
            .search
            .upsert_bookmark(&view)
            .await
            .expect("upsert meili");

        let app = app(state, true);
        let (status, body) = json_status(
            app,
            tools_call(
                &key,
                "search_bookmarks",
                json!({ "q": needle, "limit": 20 }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        assert!(body.get("error").is_none(), "rpc error: {body}");
        let payload = tool_payload(&body);
        let bookmarks = payload["bookmarks"].as_array().expect("bookmarks array");
        assert!(
            bookmarks.iter().any(|b| b["id"] == view.id.to_string()),
            "expected hit for {needle}: {payload}"
        );
    }

    #[tokio::test]
    async fn search_empty_q_browses_folder_tree() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let owner = verified_user(&state.pool, "browse").await;
        let key = api_key_for(&state.pool, &owner).await;
        let root = category(&state.pool, &owner, "Root", None).await;
        let child = category(&state.pool, &owner, "Child", Some(root)).await;
        bookmark(&state.pool, &owner, "In root", Some(root), &[], None).await;
        bookmark(&state.pool, &owner, "In child", Some(child), &[], None).await;

        let app = app(state, true);
        let (status, body) = json_status(
            app,
            tools_call(
                &key,
                "search_bookmarks",
                json!({ "folder": root.to_string() }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        assert!(body.get("error").is_none(), "rpc error: {body}");
        let payload = tool_payload(&body);
        let titles: Vec<&str> = payload["bookmarks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|b| b["title"].as_str())
            .collect();
        assert!(titles.contains(&"In root"), "{titles:?}");
        assert!(titles.contains(&"In child"), "{titles:?}");
    }

    #[tokio::test]
    async fn search_bookmarks_rejects_vault_folder() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let owner = verified_user(&state.pool, "search-vault-folder").await;
        let key = api_key_for(&state.pool, &owner).await;
        let locked = category(&state.pool, &owner, "Locked", None).await;
        bookmark(&state.pool, &owner, "Secret", Some(locked), &[], None).await;
        let (vault, _, _) =
            db::create_vault(&state.pool, owner.id, "Search Vault", "vault-passphrase")
                .await
                .unwrap();
        sqlx::query("UPDATE categories SET is_vault = TRUE, vault_id = $2 WHERE id = $1")
            .bind(locked)
            .bind(vault.id)
            .execute(&state.pool)
            .await
            .unwrap();

        let app = app(state, true);
        let (status, body) = json_status(
            app,
            tools_call(
                &key,
                "search_bookmarks",
                json!({ "folder": locked.to_string() }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        let msg = tool_error_message(&body);
        assert!(
            msg.contains("vault folders can't be browsed via MCP"),
            "expected vault folder tool error, got: {body}"
        );
    }

    #[tokio::test]
    async fn get_bookmark_hides_vault() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let owner = verified_user(&state.pool, "vault").await;
        let key = api_key_for(&state.pool, &owner).await;
        let (vault, vault_key, _) =
            db::create_vault(&state.pool, owner.id, "MCP Vault", "vault-passphrase")
                .await
                .unwrap();
        let mut keyring = VaultKeyring::default();
        keyring.insert(vault.id, vault_key);
        let view = db::create_bookmark(
            &state.pool,
            owner.id,
            CreateBookmark {
                url: format!("https://vault.mcp.example/{}", Uuid::new_v4()),
                title: Some("Secret vault link".into()),
                vault_id: Some(vault.id),
                ..Default::default()
            },
            &keyring,
        )
        .await
        .unwrap();
        assert!(view.is_vault);

        let app = app(state, true);
        let (status, body) = json_status(
            app,
            tools_call(&key, "get_bookmark", json!({ "id": view.id.to_string() })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        let msg = tool_error_message(&body);
        assert!(
            msg.contains("vault not available via MCP"),
            "expected vault tool error, got: {body}"
        );
        let payload = tool_payload(&body);
        assert!(
            payload.get("url").is_none() || payload.is_null(),
            "must not return vault body: {payload}"
        );
    }

    #[tokio::test]
    async fn get_bookmark_omits_others_only_me_notes() {
        let Some((state, _dir)) = state().await else {
            return;
        };
        let owner = verified_user(&state.pool, "notes-owner").await;
        let reader = verified_user(&state.pool, "notes-reader").await;
        let reader_key = api_key_for(&state.pool, &reader).await;
        let folder = category(&state.pool, &owner, "Shared", None).await;
        let view = bookmark(
            &state.pool,
            &owner,
            "Shared link",
            Some(folder),
            &[],
            Some("owner-only notes"),
        )
        .await;
        db::create_share(
            &state.pool,
            owner.id,
            folder,
            CreateCategoryShare {
                email: reader.email.clone(),
                access: ShareAccess::Read,
            },
        )
        .await
        .unwrap();
        for invite in db::list_share_invites(&state.pool, reader.id)
            .await
            .unwrap()
        {
            db::accept_share_invite(&state.pool, reader.id, invite.id)
                .await
                .unwrap();
        }

        let app = app(state, true);
        let (status, body) = json_status(
            app,
            tools_call(
                &reader_key,
                "get_bookmark",
                json!({ "id": view.id.to_string() }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        assert!(body.get("error").is_none(), "rpc error: {body}");
        let payload = tool_payload(&body);
        assert_eq!(payload["id"], view.id.to_string());
        assert_eq!(payload["title"], "Shared link");
        assert!(
            payload["notes"].is_null(),
            "invitee must not see owner notes: {payload}"
        );
    }
}
