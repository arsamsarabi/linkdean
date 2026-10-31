mod admin;
mod auth;
mod bookmarks;
mod categories;
mod dashboards;
mod health;
mod import_export;
mod search;
mod tags;
mod trash;
mod users;
mod vault;

use crate::state::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(admin::router())
        .merge(users::router())
        .merge(categories::router())
        .merge(bookmarks::router())
        .merge(dashboards::router())
        .merge(tags::router())
        .merge(trash::router())
        .merge(search::router())
        .merge(import_export::router())
        .merge(vault::router())
}
