//! Archived copies: who may queue them, the queue itself, quotas and vault re-keying.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.
use linkdean_core::{
    ArchiveFormat::{File, Html, Pdf},
    ArchiveStatus, CreateBookmark, CreateCategory, CreateCategoryShare, Error, ShareAccess,
    UpdateSettings, User, VaultKeyring,
};
use linkdean_db::{self as db, StoredCopy};
use sqlx::PgPool;
use uuid::Uuid;

/// The queue and platform settings are shared, so these tests run one at a time.
static GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping archive tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@archives.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user");
    db::mark_email_verified(pool, user.id)
        .await
        .expect("verify email");
    user
}

async fn folder(pool: &PgPool, owner: &User) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Archive-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn bookmark(pool: &PgPool, owner: &User, category: Option<Uuid>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://archive.example/{}/", Uuid::new_v4()),
            title: Some("Page".into()),
            category_id: category,
            category_ids: category.into_iter().collect(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn share(pool: &PgPool, owner: &User, category: Uuid, invitee: &User, access: ShareAccess) {
    db::create_share(
        pool,
        owner.id,
        category,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access,
        },
    )
    .await
    .expect("share");
    for invite in db::list_share_invites(pool, invitee.id).await.expect("inbox") {
        db::accept_share_invite(pool, invitee.id, invite.id)
            .await
            .expect("accept");
    }
}

/// Claims until it reaches `id`, failing older work left by other runs on the way.
async fn claim(pool: &PgPool, id: Uuid) -> Vec<linkdean_core::ArchiveFormat> {
    loop {
        let (got, formats) = db::claim_next_archive(pool)
            .await
            .expect("claim")
            .expect("queued work");
        if got == id {
            return formats;
        }
        db::fail_archive(pool, got, &formats, "drained by a test")
            .await
            .expect("drain");
    }
}

fn copy(format: linkdean_core::ArchiveFormat, name: &str, bytes: i64) -> StoredCopy {
    StoredCopy {
        format,
        file_name: name.into(),
        bytes,
        content_type: "text/html".into(),
        final_url: Some("https://archive.example/final".into()),
        key_vault_id: None,
        nonce: None,
    }
}

#[tokio::test]
async fn owner_and_write_invitees_archive_and_readers_cannot() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _gate = GATE.lock().await;
    let owner = person(&pool, "owner").await;
    let writer = person(&pool, "writer").await;
    let reader = person(&pool, "reader").await;
    let stranger = person(&pool, "stranger").await;
    let f = folder(&pool, &owner).await;
    share(&pool, &owner, f, &writer, ShareAccess::Write).await;
    share(&pool, &owner, f, &reader, ShareAccess::Read).await;
    let id = bookmark(&pool, &owner, Some(f)).await;
    let none = VaultKeyring::default();

    let views = db::request_archive(&pool, owner.id, id, &[Pdf, Html], &none)
        .await
        .expect("owner");
    assert_eq!(views.len(), 2);
    assert!(views.iter().all(|v| v.status == ArchiveStatus::Queued && !v.available));

    db::request_archive(&pool, writer.id, id, &[Html], &none)
        .await
        .expect("writer");
    assert!(matches!(
        db::request_archive(&pool, reader.id, id, &[Html], &none).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::request_archive(&pool, stranger.id, id, &[Html], &none).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::delete_archives(&pool, reader.id, id).await,
        Err(Error::Forbidden)
    ));
    assert!(db::request_archive(&pool, owner.id, id, &[File], &none)
        .await
        .is_err());
}

#[tokio::test]
async fn the_queue_keeps_old_copies_until_a_new_one_lands() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _gate = GATE.lock().await;
    let owner = person(&pool, "queue").await;
    let id = bookmark(&pool, &owner, None).await;
    let none = VaultKeyring::default();

    db::request_archive(&pool, owner.id, id, &[Html, Pdf], &none)
        .await
        .expect("request");
    assert_eq!(claim(&pool, id).await, vec![Html, Pdf]);
    let unused = db::finish_archive(&pool, id, &[copy(Html, "html-1", 100), copy(Pdf, "pdf-1", 50)])
        .await
        .expect("finish");
    assert!(unused.is_empty());
    assert_eq!(db::archive_bytes_for_owner(&pool, owner.id).await.unwrap(), 150);

    // Archive again: the capture fails, so both earlier copies stay, with the reason.
    db::request_archive(&pool, owner.id, id, &[Html, Pdf], &none)
        .await
        .expect("again");
    claim(&pool, id).await;
    db::fail_archive(&pool, id, &[Html, Pdf], "The page is gone (HTTP 404)")
        .await
        .expect("fail");
    let views = db::list_archives(&pool, id).await.expect("list");
    assert!(views.iter().all(|v| v.status == ArchiveStatus::Done && v.available));
    assert!(views
        .iter()
        .all(|v| v.error.as_deref() == Some("The page is gone (HTTP 404)")));

    // A restart while running puts the work back in the queue.
    db::request_archive(&pool, owner.id, id, &[Html], &none)
        .await
        .expect("third");
    claim(&pool, id).await;
    assert!(db::requeue_running_archives(&pool).await.unwrap() >= 1);
    assert_eq!(claim(&pool, id).await, vec![Html]);
    let unused = db::finish_archive(&pool, id, &[copy(Html, "html-2", 120)])
        .await
        .expect("replace");
    assert_eq!(unused, vec!["html-1".to_string()]);

    // The URL now serves a file: it replaces both page copies.
    db::request_archive(&pool, owner.id, id, &[Html, Pdf], &none)
        .await
        .expect("fourth");
    claim(&pool, id).await;
    let mut unused = db::finish_archive(&pool, id, &[copy(File, "file-1", 10)])
        .await
        .expect("file");
    unused.sort();
    assert_eq!(unused, vec!["html-2".to_string(), "pdf-1".to_string()]);
    let views = db::list_archives(&pool, id).await.expect("list");
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].format, File);

    // Deleting while a capture runs throws the capture away.
    db::request_archive(&pool, owner.id, id, &[Html], &none)
        .await
        .expect("fifth");
    claim(&pool, id).await;
    assert_eq!(
        db::delete_archives(&pool, owner.id, id).await.unwrap(),
        vec!["file-1".to_string()]
    );
    assert_eq!(
        db::finish_archive(&pool, id, &[copy(Html, "late", 5)])
            .await
            .unwrap(),
        vec!["late".to_string()]
    );
    assert!(db::list_archives(&pool, id).await.unwrap().is_empty());
}

#[tokio::test]
async fn views_summarise_copies_for_badges() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _gate = GATE.lock().await;
    let owner = person(&pool, "summary").await;
    let id = bookmark(&pool, &owner, None).await;
    let none = VaultKeyring::default();
    let view = |pool: PgPool, owner: Uuid| async move {
        let raw = db::get_bookmark_raw(&pool, owner, id).await.unwrap();
        db::bookmark_to_view(&pool, raw, &VaultKeyring::default(), owner)
            .await
            .unwrap()
            .unwrap()
    };
    assert!(view(pool.clone(), owner.id).await.archive.is_none());

    db::request_archive(&pool, owner.id, id, &[Html], &none)
        .await
        .unwrap();
    let pending = view(pool.clone(), owner.id).await.archive.unwrap();
    assert!(pending.pending && pending.formats.is_empty());

    claim(&pool, id).await;
    db::finish_archive(&pool, id, &[copy(Html, "h", 1)])
        .await
        .unwrap();
    let done = view(pool.clone(), owner.id).await.archive.unwrap();
    assert_eq!(done.formats, vec![Html]);
    assert!(!done.pending && !done.failed && done.captured_at.is_some());
}

#[tokio::test]
async fn quota_switch_and_automatic_archives() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _gate = GATE.lock().await;
    let owner = person(&pool, "quota").await;
    let id = bookmark(&pool, &owner, None).await;
    let none = VaultKeyring::default();
    db::request_archive(&pool, owner.id, id, &[Html], &none)
        .await
        .unwrap();
    claim(&pool, id).await;
    db::finish_archive(&pool, id, &[copy(Html, "q", 1_000)])
        .await
        .unwrap();

    db::set_archive_settings(&pool, None, Some(Some(1_000)), None)
        .await
        .unwrap();
    let refused = db::request_archive(&pool, owner.id, id, &[Html], &none).await;
    assert!(matches!(
        refused,
        Err(Error::QuotaExceeded {
            meter: "archive",
            limit: 1_000,
            used: 1_000
        })
    ));

    // Automatic archiving with no room records why, instead of queueing.
    db::update_user_settings(
        &pool,
        owner.id,
        &UpdateSettings {
            show_hidden: None,
            vault_search_mode: None,
            theme: None,
            vault_auto_lock_minutes: None,
            default_landing: None,
            library_view: None,
            trash_retention_days: None,
            show_deleted_in_library: None,
            show_vault_in_library: None,
            vault_delete_mode: None,
            show_keyboard_hints: None,
            archive_new: Some(true),
            archive_formats: Some(vec![Pdf]),
        },
    )
    .await
    .unwrap();
    let fresh = bookmark(&pool, &owner, None).await;
    assert!(!db::archive_new_bookmark(&pool, owner.id, fresh).await.unwrap());
    let views = db::list_archives(&pool, fresh).await.unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].format, Pdf);
    assert_eq!(views[0].status, ArchiveStatus::Failed);
    assert_eq!(views[0].error.as_deref(), Some(db::ARCHIVE_STORAGE_FULL));

    db::set_archive_settings(&pool, None, Some(None), None)
        .await
        .unwrap();
    let roomy = bookmark(&pool, &owner, None).await;
    assert!(db::archive_new_bookmark(&pool, owner.id, roomy).await.unwrap());

    db::set_archive_settings(&pool, Some(false), None, None)
        .await
        .unwrap();
    let off = db::request_archive(&pool, owner.id, id, &[Html], &none).await;
    db::set_archive_settings(&pool, Some(true), None, None)
        .await
        .unwrap();
    assert!(matches!(off, Err(Error::BadRequest(_))));
    assert!(db::set_archive_settings(&pool, None, None, Some(10))
        .await
        .is_err());
    claim(&pool, roomy).await;
    db::fail_archive(&pool, roomy, &[Pdf], "cleanup").await.unwrap();
}

#[tokio::test]
async fn vault_copies_need_the_vault_and_follow_moves() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _gate = GATE.lock().await;
    let owner = person(&pool, "vault").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Archive vault", "vault-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    let id = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://secret.example/".into(),
            title: Some("Secret".into()),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("vault bookmark")
    .id;

    assert!(matches!(
        db::request_archive(&pool, owner.id, id, &[Html], &VaultKeyring::default()).await,
        Err(Error::VaultLocked)
    ));
    db::request_archive(&pool, owner.id, id, &[Html], &keyring)
        .await
        .expect("unlocked");
    claim(&pool, id).await;
    // Stored in plain by mistake (or before a move in): the re-key pass finds it.
    db::finish_archive(&pool, id, &[copy(Html, "plain", 10)])
        .await
        .unwrap();
    let pending = db::archives_needing_rekey(&pool, owner.id).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].1, Some(vault.id));

    assert!(!db::set_archive_key(&pool, id, Html, "other", "sealed", Some(vault.id), Some(vec![1]))
        .await
        .unwrap());
    assert!(db::set_archive_key(&pool, id, Html, "plain", "sealed", Some(vault.id), Some(vec![1]))
        .await
        .unwrap());
    assert!(db::archives_needing_rekey(&pool, owner.id).await.unwrap().is_empty());
    let row = db::archive_row(&pool, id, Html).await.unwrap().unwrap();
    assert_eq!(row.file_name.as_deref(), Some("sealed"));
    assert!(row.final_url.is_none(), "a vault copy keeps no plain address");
}
