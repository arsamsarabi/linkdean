mod activity;
mod admin;
mod agent_skills;
mod api_keys;
mod assets;
pub(crate) mod auth;
mod bookmarks;
mod categories;
mod dashboards;
mod dev;
mod health;
mod import_export;
mod notifications;
mod public;
mod reports;
mod search;
mod share_invites;
mod social;
mod tags;
mod trash;
mod users;
mod vault;

use crate::state::AppState;
use axum::Router;

/// Every `/api` route. The development-only pages join only when `dev_routes` is on, and
/// experimental features only when their flag is on (otherwise they answer 404).
pub fn api_router(dev_routes: bool, agent_skills: bool) -> Router<AppState> {
    let mut routes = router();
    if dev_routes {
        routes = routes.merge(dev::router());
    }
    if agent_skills {
        routes = routes.merge(agent_skills::router());
    }
    routes
}

fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(admin::router())
        .merge(api_keys::router())
        .merge(activity::router())
        .merge(notifications::router())
        .merge(share_invites::router())
        .merge(users::router())
        .merge(categories::router())
        .merge(bookmarks::router())
        .merge(dashboards::router())
        .merge(social::router())
        .merge(tags::router())
        .merge(trash::router())
        .merge(search::router())
        .merge(import_export::router())
        .merge(vault::router())
        .merge(public::router())
        .merge(reports::router())
}
