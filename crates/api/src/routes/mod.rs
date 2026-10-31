mod admin;
mod auth;
mod bookmarks;
mod categories;
mod health;
mod import_export;
mod search;
mod tags;
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
        .merge(tags::router())
        .merge(search::router())
        .merge(import_export::router())
        .merge(vault::router())
}
