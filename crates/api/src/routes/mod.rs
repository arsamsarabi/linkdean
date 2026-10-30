mod auth_routes;
mod bookmarks;
mod categories;
mod health;
mod import_export;
mod search;
mod tags;
mod users;
mod vault;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth_routes::router())
        .merge(users::router())
        .merge(categories::router())
        .merge(bookmarks::router())
        .merge(tags::router())
        .merge(search::router())
        .merge(import_export::router())
        .merge(vault::router())
}
