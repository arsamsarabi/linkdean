//! `/terms`, `/privacy` and `/acceptable-use` on the app redirect to the operator's pages, so
//! links in sent emails and store listings keep working. Unset URLs fall through to the app's
//! not-found page.

use crate::config::Legal;
use axum::response::Redirect;
use axum::routing::{get, MethodRouter};
use axum::Router;

pub fn redirect_router(legal: &Legal) -> Router {
    let mut router = Router::new();
    if let Some(url) = &legal.terms_url {
        router = router
            .route("/terms", to(url.clone()))
            .route("/acceptable-use", to(format!("{url}#acceptable-use")));
    }
    if let Some(url) = &legal.privacy_url {
        router = router.route("/privacy", to(url.clone()));
    }
    router
}

fn to(url: String) -> MethodRouter {
    get(move || async move { Redirect::temporary(&url) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt;

    async fn get_path(router: Router, path: &str) -> (StatusCode, Option<String>) {
        let res = router
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let location = res
            .headers()
            .get(header::LOCATION)
            .map(|v| v.to_str().unwrap().to_string());
        (res.status(), location)
    }

    #[tokio::test]
    async fn redirects_only_the_pages_that_are_set() {
        let legal = Legal {
            terms_version: Some("2026-11-15".into()),
            terms_url: Some("https://example.com/terms".into()),
            privacy_url: None,
        };
        let (status, location) = get_path(redirect_router(&legal), "/terms").await;
        assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(location.as_deref(), Some("https://example.com/terms"));
        let (_, location) = get_path(redirect_router(&legal), "/acceptable-use").await;
        assert_eq!(
            location.as_deref(),
            Some("https://example.com/terms#acceptable-use")
        );
        let (status, _) = get_path(redirect_router(&legal), "/privacy").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
