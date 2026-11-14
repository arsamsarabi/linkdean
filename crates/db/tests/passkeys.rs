//! Passkeys and WebAuthn ceremony state.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{Error, User};
use linkdean_db::{self as db, WebauthnPurpose};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping passkey tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool) -> User {
    let email = format!("pk-{}@passkeys.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

fn cred() -> Vec<u8> {
    Uuid::new_v4().as_bytes().to_vec()
}

#[tokio::test]
async fn a_challenge_is_single_use_and_bound_to_its_purpose_and_account() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool).await;
    let other = person(&pool).await;
    let state = json!({ "rs": "x" });

    let id = db::create_webauthn_challenge(&pool, Some(user.id), WebauthnPurpose::Register, &state)
        .await
        .expect("create");
    // Wrong purpose or another account: nothing, and the challenge survives.
    assert_eq!(
        db::take_webauthn_challenge(&pool, id, WebauthnPurpose::Login, Some(user.id))
            .await
            .expect("take"),
        None
    );
    assert_eq!(
        db::take_webauthn_challenge(&pool, id, WebauthnPurpose::Register, Some(other.id))
            .await
            .expect("take"),
        None
    );
    assert_eq!(
        db::take_webauthn_challenge(&pool, id, WebauthnPurpose::Register, Some(user.id))
            .await
            .expect("take"),
        Some(state.clone())
    );
    assert_eq!(
        db::take_webauthn_challenge(&pool, id, WebauthnPurpose::Register, Some(user.id))
            .await
            .expect("take again"),
        None
    );

    // Sign-in ceremonies have no account; an expired one is gone and purged.
    let login = db::create_webauthn_challenge(&pool, None, WebauthnPurpose::Login, &state)
        .await
        .expect("create login");
    sqlx::query(
        "UPDATE webauthn_challenges SET expires_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(login)
    .execute(&pool)
    .await
    .expect("expire");
    assert_eq!(
        db::take_webauthn_challenge(&pool, login, WebauthnPurpose::Login, None)
            .await
            .expect("take expired"),
        None
    );
    assert!(
        db::purge_expired_webauthn_challenges(&pool)
            .await
            .expect("purge")
            >= 1
    );
}

#[tokio::test]
async fn passkeys_are_listed_renamed_used_and_removed_by_their_owner_only() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool).await;
    let other = person(&pool).await;
    let id_bytes = cred();
    let pk = db::insert_passkey(
        &pool,
        user.id,
        &id_bytes,
        "  MacBook  ",
        &json!({ "cred": 1 }),
    )
    .await
    .expect("insert");
    assert_eq!(pk.name, "MacBook");

    // One credential belongs to one account.
    let dup = db::insert_passkey(&pool, other.id, &id_bytes, "Stolen", &json!({}))
        .await
        .unwrap_err();
    assert!(matches!(dup, Error::Conflict(_)), "{dup}");
    let blank = db::insert_passkey(&pool, user.id, &cred(), "   ", &json!({}))
        .await
        .unwrap_err();
    assert!(matches!(blank, Error::BadRequest(_)), "{blank}");

    assert_eq!(
        db::find_passkey(&pool, user.id, &id_bytes)
            .await
            .expect("find"),
        Some((pk.id, json!({ "cred": 1 })))
    );
    assert_eq!(
        db::find_passkey(&pool, other.id, &id_bytes)
            .await
            .expect("find"),
        None
    );
    assert_eq!(
        db::passkey_credential_ids(&pool, user.id)
            .await
            .expect("ids"),
        vec![id_bytes.clone()]
    );

    db::record_passkey_use(&pool, pk.id, &json!({ "cred": 2 }))
        .await
        .expect("use");
    let listed = db::list_passkeys(&pool, user.id).await.expect("list");
    assert_eq!(listed.len(), 1);
    assert!(listed[0].last_used_at.is_some());

    let err = db::rename_passkey(&pool, other.id, pk.id, "Mine now")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound), "{err}");
    db::rename_passkey(&pool, user.id, pk.id, "Phone")
        .await
        .expect("rename");
    assert_eq!(
        db::list_passkeys(&pool, user.id).await.expect("list")[0].name,
        "Phone"
    );

    let err = db::delete_passkey(&pool, other.id, pk.id)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound), "{err}");
    db::delete_passkey(&pool, user.id, pk.id)
        .await
        .expect("delete");
    assert!(db::list_passkeys(&pool, user.id)
        .await
        .expect("list")
        .is_empty());
}

#[tokio::test]
async fn an_account_holds_a_limited_number_of_passkeys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool).await;
    for i in 0..db::MAX_PASSKEYS_PER_USER {
        db::insert_passkey(&pool, user.id, &cred(), &format!("Key {i}"), &json!({}))
            .await
            .expect("insert");
    }
    let err = db::insert_passkey(&pool, user.id, &cred(), "One more", &json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "{err}");
}
