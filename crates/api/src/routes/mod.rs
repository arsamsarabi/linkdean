mod activity;
mod admin;
mod agent_skills;
mod api_keys;
mod archives;
mod assets;
pub(crate) mod auth;
mod billing;
mod bookmarks;
mod categories;
mod dashboards;
mod dev;
mod email_unsubscribe;
mod health;
mod import_export;
mod notifications;
mod organization_invite_links;
mod organization_invites;
mod organizations;
mod public;
mod reports;
mod saved_searches;
mod scim;
mod search;
mod share_invites;
mod social;
mod tags;
mod trash;
mod users;
mod vault;

use crate::state::AppState;
use axum::Router;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{InfoBuilder, OpenApi, ServerBuilder};
use utoipa_axum::router::OpenApiRouter;

/// Every `/api` route. The development-only pages join only when `dev_routes` is on, and
/// experimental features only when their flag is on (otherwise they answer 404).
pub fn api_router(dev_routes: bool, agent_skills: bool) -> Router<AppState> {
    let (mut routes, _) = documented(agent_skills).split_for_parts();
    // Stripe webhook needs a raw body; keep it off the OpenAPI router.
    routes = routes.merge(billing::webhook_router());
    // MCP sits outside OpenAPI; same experimental flag as the Agent Skill zip.
    if agent_skills {
        routes = routes.merge(crate::mcp::router());
    }
    if dev_routes {
        routes = routes.merge(dev::router());
    }
    routes
}

/// The OpenAPI description of every documented route, experimental ones included.
pub fn openapi() -> OpenApi {
    let (_, mut api) = documented(true).split_for_parts();
    api.info = InfoBuilder::new()
        .title("Linkdean API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some(API_DESCRIPTION))
        .build();
    api.servers = Some(vec![ServerBuilder::new()
        .url("/api")
        .description(Some("This instance"))
        .build()]);
    let components = api.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
        "api_key",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .description(Some(
                    "An API key (`ldk_…`) from Config options → API keys, or a sign-in \
                     session token, as `Authorization: Bearer <token>`.",
                ))
                .build(),
        ),
    );
    components.add_security_scheme(
        "session_token",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .description(Some(
                    "A sign-in session token as `Authorization: Bearer <token>`. API keys \
                     are refused here.",
                ))
                .build(),
        ),
    );
    components.add_security_scheme(
        "session_cookie",
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            "linkdean_session",
            "The browser's sign-in cookie. Writes also need the `X-Linkdean-Client` header.",
        ))),
    );
    api
}

const API_DESCRIPTION: &str = "The HTTP API behind the Linkdean web app, browser extension \
and integrations. Every error answers `{ code, message, details?, error }`: match on `code`, \
show `message`. Successful JSON reads carry an `ETag`; send it back in `If-None-Match` to get \
`304 Not Modified`. Every response has an `X-Request-Id` to quote in bug reports.";

fn documented(agent_skills: bool) -> OpenApiRouter<AppState> {
    let mut routes = OpenApiRouter::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(billing::router())
        .merge(admin::router())
        .merge(api_keys::router())
        .merge(activity::router())
        .merge(notifications::router())
        .merge(organizations::router())
        .merge(organization_invite_links::router())
        .merge(organization_invites::router())
        .merge(scim::router())
        .merge(share_invites::router())
        .merge(users::router())
        .merge(categories::router())
        .merge(bookmarks::router())
        .merge(archives::router())
        .merge(dashboards::router())
        .merge(social::router())
        .merge(tags::router())
        .merge(trash::router())
        .merge(search::router())
        .merge(saved_searches::router())
        .merge(import_export::router())
        .merge(email_unsubscribe::router())
        .merge(vault::router())
        .merge(public::router())
        .merge(reports::router())
        .merge(crate::metrics::router())
        .routes(utoipa_axum::routes!(openapi_json));
    if agent_skills {
        routes = routes.merge(agent_skills::router());
    }
    routes
}

/// This API's OpenAPI 3.1 description.
#[utoipa::path(
    get,
    operation_id = "api_openapi_json",
    path = "/openapi.json",
    tag = "meta",
    responses((status = 200, description = "OpenAPI 3.1 document", content_type = "application/json")),
)]
async fn openapi_json() -> axum::Json<OpenApi> {
    axum::Json(openapi())
}

#[cfg(test)]
mod route_matrix_tests;

#[cfg(test)]
mod openapi_tests {
    use super::openapi;
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn spec() -> Value {
        serde_json::to_value(openapi()).unwrap()
    }

    /// The committed copy, which the docs site publishes.
    fn committed_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/api/openapi.json")
    }

    #[test]
    fn committed_spec_is_current() {
        let fresh = serde_json::to_string_pretty(&spec()).unwrap() + "\n";
        if std::env::var("UPDATE_OPENAPI").is_ok() {
            std::fs::write(committed_path(), &fresh).unwrap();
            return;
        }
        let committed = std::fs::read_to_string(committed_path()).unwrap_or_default();
        assert!(
            committed == fresh,
            "docs/api/openapi.json is stale: run `UPDATE_OPENAPI=1 cargo test -p linkdean-api committed_spec_is_current`"
        );
    }

    fn refs(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(m) => {
                if let Some(Value::String(r)) = m.get("$ref") {
                    out.insert(r.clone());
                }
                m.values().for_each(|x| refs(x, out));
            }
            Value::Array(a) => a.iter().for_each(|x| refs(x, out)),
            _ => {}
        }
    }

    #[test]
    fn every_reference_resolves() {
        let s = spec();
        let mut all = BTreeSet::new();
        refs(&s, &mut all);
        let schemas = &s["components"]["schemas"];
        let missing: Vec<_> = all
            .iter()
            .filter(|r| {
                let name = r.trim_start_matches("#/components/schemas/");
                schemas.get(name).is_none()
            })
            .collect();
        assert!(missing.is_empty(), "unresolved: {missing:?}");
    }

    #[test]
    fn every_operation_documents_errors_and_path_parameters() {
        let s = spec();
        let mut ops = 0;
        for (path, item) in s["paths"].as_object().unwrap() {
            let template: BTreeSet<String> = path
                .split('/')
                .filter_map(|seg| seg.strip_prefix('{').and_then(|x| x.strip_suffix('}')))
                .map(String::from)
                .collect();
            for (method, op) in item.as_object().unwrap() {
                if method == "parameters" {
                    continue;
                }
                ops += 1;
                let where_ = format!("{method} {path}");
                if path != "/openapi.json" {
                    assert!(op["responses"].get("4XX").is_some(), "{where_}: no 4XX");
                }
                let declared: Vec<String> = op["parameters"]
                    .as_array()
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| p["in"] == "path")
                            .map(|p| p["name"].as_str().unwrap().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let unique: BTreeSet<String> = declared.iter().cloned().collect();
                assert_eq!(
                    unique.len(),
                    declared.len(),
                    "{where_}: duplicate path params"
                );
                assert_eq!(unique, template, "{where_}: path params");
                assert!(
                    op["tags"].as_array().is_some_and(|t| !t.is_empty()),
                    "{where_}: no tag"
                );
            }
        }
        assert!(ops >= 190, "only {ops} operations documented");
        let ids: Vec<&str> = s["paths"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|i| i.as_object().unwrap().values())
            .filter_map(|op| op["operationId"].as_str())
            .collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), ops, "every operation has an operationId");
        assert_eq!(unique.len(), ids.len(), "operationIds are unique");
    }
}
