//! First use of a default dashboard.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_db::{self as db};
use uuid::Uuid;

#[tokio::test]
async fn parallel_first_requests_share_one_default_dashboard() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping dashboard tests: DATABASE_URL unset");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    for _ in 0..20 {
        let email = format!("dash-{}@dashboards.test", Uuid::new_v4());
        let user = db::create_user(&pool, &email, Some("account-password"), None, None, None)
            .await
            .expect("user");
        let requests = (0..8).map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move { db::ensure_default_dashboard(&pool, user.id).await })
        });
        let mut ids = Vec::new();
        for request in requests.collect::<Vec<_>>() {
            ids.push(request.await.expect("join").expect("dashboard").id);
        }
        ids.dedup();
        assert_eq!(ids.len(), 1, "every request gets the same dashboard");
    }
}
