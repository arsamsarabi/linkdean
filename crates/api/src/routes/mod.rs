mod activity;
mod admin;
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
mod search;
mod share_invites;
mod social;
mod tags;
mod trash;
mod users;
mod vault;

use crate::state::AppState;
use axum::Router;

/// Every `/api` route. The development-only pages join only when `dev_routes` is on.
pub fn api_router(dev_routes: bool) -> Router<AppState> {
    let routes = router();
    if dev_routes {
        routes.merge(dev::router())
    } else {
        routes
    }
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
}
