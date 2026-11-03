//! Pending single sign-on links: expiry, single use, binding to the account that holds the
//! address, one identity per account and per subject, verification and unlinking.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::crypto::hash_token;
use linkdean_core::{Error, User};
use linkdean_db as db;
use sqlx::PgPool;
use uuid::Uuid;

const ISSUER: &str = "https://sso.link.test/default";

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping pending oidc link tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn subject() -> String {
    format!("sub-{}", Uuid::new_v4())
}

/// A new, unverified password account.
async fn password_user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@link.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn reload(pool: &PgPool, id: Uuid) -> User {
    db::find_user_by_id(pool, id)
        .await
        .expect("find")
        .expect("exists")
}

/// A pending link for `email`, which the provider reports as `verified`.
async fn pending(pool: &PgPool, sub: &str, email: &str, verified: bool) -> String {
    db::create_pending_oidc_link(pool, ISSUER, sub, email, verified)
        .await
        .expect("pending link")
}

#[tokio::test]
async fn a_link_attaches_the_identity_once() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "once").await;
    let sub = subject();
    // The provider may report the address in another case.
    let token = pending(&pool, &sub, &u.email.to_uppercase(), false).await;
    assert_eq!(token.len(), 64);

    let peeked = db::peek_pending_oidc_link(&pool, &token)
        .await
        .expect("peek")
        .expect("live");
    assert_eq!(peeked.issuer, ISSUER);
    assert_eq!(peeked.subject, sub);
    assert!(!peeked.email_verified);

    let linked = db::consume_pending_oidc_link(&pool, &token, u.id)
        .await
        .expect("consume")
        .expect("linked");
    assert_eq!(linked.subject, sub);
    let found = db::find_user_by_oidc(&pool, ISSUER, &sub)
        .await
        .expect("find")
        .expect("linked user");
    assert_eq!(found.id, u.id);
    // The provider did not vouch for the address, so the account stays unverified.
    assert!(reload(&pool, u.id).await.email_verified_at.is_none());

    // Used: neither peek nor a second consume sees it.
    assert!(db::peek_pending_oidc_link(&pool, &token)
        .await
        .expect("peek")
        .is_none());
    assert!(db::consume_pending_oidc_link(&pool, &token, u.id)
        .await
        .expect("consume again")
        .is_none());
}

#[tokio::test]
async fn only_its_own_token_works() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "token").await;
    let _token = pending(&pool, &subject(), &u.email, false).await;
    let wrong = hex::encode([7u8; 32]);
    assert!(db::peek_pending_oidc_link(&pool, &wrong)
        .await
        .expect("peek")
        .is_none());
    assert!(db::consume_pending_oidc_link(&pool, &wrong, u.id)
        .await
        .expect("consume")
        .is_none());
    assert!(reload(&pool, u.id).await.oidc_subject.is_none());
}

#[tokio::test]
async fn an_expired_link_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "expired").await;
    let token = pending(&pool, &subject(), &u.email, true).await;
    sqlx::query(
        "UPDATE pending_oidc_links SET expires_at = now() - interval '1 second' \
         WHERE token_hash = $1",
    )
    .bind(hash_token(&token))
    .execute(&pool)
    .await
    .expect("expire");
    assert!(db::peek_pending_oidc_link(&pool, &token)
        .await
        .expect("peek")
        .is_none());
    assert!(db::consume_pending_oidc_link(&pool, &token, u.id)
        .await
        .expect("consume")
        .is_none());
    let after = reload(&pool, u.id).await;
    assert!(after.oidc_subject.is_none());
    assert!(after.email_verified_at.is_none());
}

#[tokio::test]
async fn links_expire_after_ten_minutes() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let token = pending(&pool, &subject(), "ttl@link.test", false).await;
    let link = db::peek_pending_oidc_link(&pool, &token)
        .await
        .expect("peek")
        .expect("live");
    let left = link.expires_at - chrono::Utc::now();
    assert_eq!(db::PENDING_OIDC_LINK_MINUTES, 10);
    assert!(left <= chrono::Duration::minutes(10));
    assert!(left > chrono::Duration::minutes(9));
}

#[tokio::test]
async fn another_account_links_nothing_and_spends_the_link() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let holder = password_user(&pool, "holder").await;
    let other = password_user(&pool, "other").await;
    let sub = subject();
    let token = pending(&pool, &sub, &holder.email, true).await;

    assert!(db::consume_pending_oidc_link(&pool, &token, other.id)
        .await
        .expect("consume")
        .is_none());
    assert!(db::find_user_by_oidc(&pool, ISSUER, &sub)
        .await
        .expect("find")
        .is_none());
    assert!(reload(&pool, other.id).await.email_verified_at.is_none());

    // Spent: the holder cannot use it afterwards either.
    assert!(db::consume_pending_oidc_link(&pool, &token, holder.id)
        .await
        .expect("consume")
        .is_none());
    assert!(reload(&pool, holder.id).await.oidc_subject.is_none());
}

#[tokio::test]
async fn an_identity_belongs_to_one_account() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let first = password_user(&pool, "first").await;
    let second = password_user(&pool, "second").await;
    let sub = subject();
    db::link_oidc_identity(&pool, first.id, ISSUER, &sub)
        .await
        .expect("link first");
    // Linking the same identity again to the same account is a no-op.
    db::link_oidc_identity(&pool, first.id, ISSUER, &sub)
        .await
        .expect("relink same");

    let err = db::link_oidc_identity(&pool, second.id, ISSUER, &sub)
        .await
        .expect_err("taken");
    assert!(matches!(err, Error::Conflict(ref m) if m.contains("another account")));

    // The same through a pending link: refused, and the link is spent.
    let token = pending(&pool, &sub, &second.email, true).await;
    let err = db::consume_pending_oidc_link(&pool, &token, second.id)
        .await
        .expect_err("taken");
    assert!(matches!(err, Error::Conflict(_)));
    assert!(db::peek_pending_oidc_link(&pool, &token)
        .await
        .expect("peek")
        .is_none());
    let after = reload(&pool, second.id).await;
    assert!(after.oidc_subject.is_none());
    assert!(after.email_verified_at.is_none());
}

#[tokio::test]
async fn an_account_has_one_identity() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "single").await;
    let first = subject();
    db::link_oidc_identity(&pool, u.id, ISSUER, &first)
        .await
        .expect("link");
    let err = db::link_oidc_identity(&pool, u.id, ISSUER, &subject())
        .await
        .expect_err("second identity");
    assert!(matches!(err, Error::Conflict(_)));

    let token = pending(&pool, &subject(), &u.email, true).await;
    let err = db::consume_pending_oidc_link(&pool, &token, u.id)
        .await
        .expect_err("second identity");
    assert!(matches!(err, Error::Conflict(_)));
    assert_eq!(
        reload(&pool, u.id).await.oidc_subject.as_deref(),
        Some(first.as_str())
    );
}

#[tokio::test]
async fn a_verified_matching_address_marks_the_account_verified() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "verified").await;
    assert!(u.email_verified_at.is_none());
    let token = pending(&pool, &subject(), &u.email, true).await;
    db::consume_pending_oidc_link(&pool, &token, u.id)
        .await
        .expect("consume")
        .expect("linked");
    assert!(reload(&pool, u.id).await.email_verified_at.is_some());
}

#[tokio::test]
async fn connecting_directly_does_not_verify() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "connect").await;
    db::link_oidc_identity(&pool, u.id, ISSUER, &subject())
        .await
        .expect("link");
    assert!(reload(&pool, u.id).await.email_verified_at.is_none());
}

#[tokio::test]
async fn unlinking_needs_the_right_local_password() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = password_user(&pool, "unlink").await;
    let sub = subject();
    db::link_oidc_identity(&pool, u.id, ISSUER, &sub)
        .await
        .expect("link");

    let err = db::unlink_oidc_identity(&pool, u.id, "wrong-password")
        .await
        .expect_err("wrong password");
    assert!(matches!(err, Error::Unauthorized));
    assert!(reload(&pool, u.id).await.oidc_subject.is_some());

    db::unlink_oidc_identity(&pool, u.id, "password-long")
        .await
        .expect("unlink");
    let after = reload(&pool, u.id).await;
    assert!(after.oidc_subject.is_none());
    assert!(after.oidc_issuer.is_none());
    assert!(db::find_user_by_oidc(&pool, ISSUER, &sub)
        .await
        .expect("find")
        .is_none());

    // Nothing left to disconnect.
    let err = db::unlink_oidc_identity(&pool, u.id, "password-long")
        .await
        .expect_err("not connected");
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn an_sso_only_account_cannot_unlink() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let sub = subject();
    let email = format!("ssoonly-{}@link.test", Uuid::new_v4());
    let u = db::create_user(&pool, &email, None, None, Some(&sub), Some(ISSUER))
        .await
        .expect("sso user");
    let err = db::unlink_oidc_identity(&pool, u.id, "anything-long")
        .await
        .expect_err("no password");
    assert!(matches!(err, Error::BadRequest(_)));
    assert_eq!(
        reload(&pool, u.id).await.oidc_subject.as_deref(),
        Some(sub.as_str())
    );
}

#[tokio::test]
async fn purge_drops_old_links_only() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let live = pending(&pool, &subject(), "purge-live@link.test", false).await;
    let old_used = pending(&pool, &subject(), "purge-used@link.test", false).await;
    let old_expired = pending(&pool, &subject(), "purge-expired@link.test", false).await;
    sqlx::query(
        "UPDATE pending_oidc_links SET used_at = now() - interval '2 days' WHERE token_hash = $1",
    )
    .bind(hash_token(&old_used))
    .execute(&pool)
    .await
    .expect("age used");
    sqlx::query(
        "UPDATE pending_oidc_links SET expires_at = now() - interval '2 days' \
         WHERE token_hash = $1",
    )
    .bind(hash_token(&old_expired))
    .execute(&pool)
    .await
    .expect("age expired");

    let purged = db::purge_old_pending_oidc_links(&pool)
        .await
        .expect("purge");
    assert!(purged >= 2);
    let left: Vec<String> =
        sqlx::query_scalar("SELECT token_hash FROM pending_oidc_links WHERE token_hash = ANY($1)")
            .bind(vec![
                hash_token(&live),
                hash_token(&old_used),
                hash_token(&old_expired),
            ])
            .fetch_all(&pool)
            .await
            .expect("left");
    assert_eq!(left, vec![hash_token(&live)]);
}
