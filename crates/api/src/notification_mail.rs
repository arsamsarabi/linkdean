//! Email for notifications and share invites.
//!
//! The database marks a new notification `email_state = 'pending'` when its recipient wants it by
//! email. While mail is on, each outbox tick claims pending rows, renders them and queues them in
//! the outbox. While mail is off, each tick discards pending rows, so turning mail on later sends
//! no backlog.
//!
//! Invites to an address (no account, or an unverified one) carry a claim token that only the
//! request that created it sees, so the share routes email those themselves.

use crate::mail::{enqueue, Queued};
use crate::rate_limit::invite_email_limit;
use crate::state::AppState;
use linkdean_core::{
    NotificationKind, NotificationSubjectType, PendingShareInviteView, PendingShareSubjectType,
    ShareAccess, User,
};
use linkdean_db::{self as db, PendingNotificationEmail};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// How many notification emails one tick renders.
const CLAIM_BATCH: i64 = 50;

/// Collapses whitespace, newlines included, so a value can go into a subject line.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn app_url(state: &AppState) -> &str {
    state.cfg.app_url.trim_end_matches('/')
}

/// Settings → Notifications in the web app.
pub fn settings_url(app_url: &str) -> String {
    format!(
        "{}/settings?tab=notifications",
        app_url.trim_end_matches('/')
    )
}

/// The link a pending invite's email carries; the web app builds the same one.
pub fn claim_url(app_url: &str, token: &str) -> String {
    format!(
        "{}/register?share={}",
        app_url.trim_end_matches('/'),
        url::form_urlencoded::byte_serialize(token.as_bytes()).collect::<String>()
    )
}

/// How invite emails name the owner: their display name, else their address.
pub fn owner_label(user: &User) -> String {
    user.display_name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&user.email)
        .to_string()
}

fn subject_noun(subject_type: NotificationSubjectType) -> Option<&'static str> {
    match subject_type {
        NotificationSubjectType::Category => Some("folder"),
        NotificationSubjectType::Dashboard => Some("dashboard"),
        _ => None,
    }
}

/// The web page a notification email links to, as a path.
fn path_for(n: &PendingNotificationEmail) -> String {
    match n.kind {
        NotificationKind::ShareInvite
        | NotificationKind::ShareAccessChanged
        | NotificationKind::ShareRevoked => "/shared".into(),
        NotificationKind::ModerationChanges => "/trash".into(),
        _ => match n.subject_type {
            NotificationSubjectType::Dashboard => format!("/?board={}", n.subject_id),
            // Folders shared with the recipient live on the Shared page; their own in the library.
            NotificationSubjectType::Category if n.kind == NotificationKind::SharedEdit => {
                "/shared".into()
            }
            NotificationSubjectType::Category => format!("/library?folder={}", n.subject_id),
            NotificationSubjectType::Bookmark => "/library".into(),
            NotificationSubjectType::User => format!("/u/{}", n.subject_id),
        },
    }
}

fn heading_for(n: &PendingNotificationEmail) -> String {
    let noun = subject_noun(n.subject_type).unwrap_or("item");
    match n.kind {
        NotificationKind::ShareInvite => format!("New invite to a shared {noun}"),
        NotificationKind::ShareAccessChanged => format!("Your access to a shared {noun} changed"),
        NotificationKind::ShareRevoked => format!("A shared {noun} was removed"),
        NotificationKind::SharedEdit => format!("Edit in a shared {noun}"),
        NotificationKind::SocialLike => "New like".into(),
        NotificationKind::SocialComment => "New comment".into(),
        NotificationKind::SocialRating => "New rating".into(),
        NotificationKind::UserFollowed => "New follower".into(),
        NotificationKind::ModerationChanges => "A moderator changed your public content".into(),
    }
}

/// The template and data for one notification email.
fn email_for(n: &PendingNotificationEmail, app_url: &str) -> (&'static str, Value) {
    let url = format!("{app_url}{}", path_for(n));
    let kind_label = n.kind.label();
    let settings = settings_url(app_url);
    if n.kind == NotificationKind::ShareInvite {
        let access = n
            .payload
            .get("access")
            .and_then(Value::as_str)
            .and_then(|a| ShareAccess::parse(a).ok())
            .map(db::access_label);
        if let (Some(noun), Some(access)) = (subject_noun(n.subject_type), access) {
            let owner = n.actor_label.as_deref().unwrap_or("Someone");
            return (
                "share_invite",
                json!({
                    "owner": one_line(owner),
                    "subject_kind": noun,
                    "name": n.subject_title,
                    "access": access,
                    "url": url,
                    "kind_label": kind_label,
                    "settings_url": settings,
                }),
            );
        }
    }
    (
        "notification",
        json!({
            "heading": one_line(&heading_for(n)),
            "message": n.body,
            "url": url,
            "kind_label": kind_label,
            "settings_url": settings,
        }),
    )
}

/// One tick while mail is on: claim pending notification emails and queue them in the outbox.
/// Returns how many it claimed.
pub async fn queue_notification_emails(state: &AppState) -> usize {
    let batch = match db::claim_pending_notification_emails(&state.pool, CLAIM_BATCH).await {
        Ok(batch) => batch,
        Err(e) => {
            tracing::warn!(error = %e, "claiming notification emails failed");
            return 0;
        }
    };
    let base = app_url(state);
    for n in &batch {
        let (template, data) = email_for(n, base);
        // Claimed rows are queued at most once: a failure here loses this email, never repeats it.
        if let Err(e) = enqueue(state, &n.recipient_email, template, &data).await {
            tracing::warn!(notification_id = %n.id, error = %e, "queueing notification email failed");
        }
    }
    batch.len()
}

/// One tick while mail is off: drop every pending notification email.
pub async fn discard_notification_emails(pool: &PgPool) {
    loop {
        match db::discard_pending_notification_emails(pool, 500).await {
            Ok(0) => return,
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = %e, "discarding notification emails failed");
                return;
            }
        }
    }
}

/// Whether a new share sent its invitee an email, for the share response.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InviteEmail {
    pub sent: bool,
    /// The owner has sent too many invite emails in the last hour; this one was not sent.
    pub limited: bool,
}

/// Adds `email_sent` (and `email_limited` when limited) to a share response.
pub fn with_invite_email<T: serde::Serialize>(outcome: &T, email: InviteEmail) -> Value {
    let mut v = serde_json::to_value(outcome).unwrap_or_else(|_| json!({}));
    if let Some(map) = v.as_object_mut() {
        map.insert("email_sent".into(), json!(email.sent));
        if email.limited {
            map.insert("email_limited".into(), json!(true));
        }
    }
    v
}

/// The clock to pass to [`invite_email`], read before creating the share. `None` with mail off.
pub async fn invite_clock(state: &AppState) -> Option<chrono::DateTime<chrono::Utc>> {
    if state.mail.is_none() {
        return None;
    }
    match db::database_now(&state.pool).await {
        Ok(now) => Some(now),
        Err(e) => {
            tracing::warn!(error = %e, "reading the database clock failed");
            None
        }
    }
}

/// Emails the invite a share route just created, within the owner's hourly limit.
///
/// - With a claim token (an address with no account, or an unverified one): sends
///   `share_invite_pending` with the claim link, unless that address's account turned share invite
///   email off. Each such share issues a new link, so a repeat share is emailed again.
/// - Without one (a verified account): the in-app notification's email is already pending; this
///   only reports it, or cancels it when the owner is over the limit. A repeat share, a declined
///   invite (refused before this) or an access change creates no notification, so no email.
///
/// Never fails the share: errors are logged and reported as not sent.
pub async fn invite_email(
    state: &AppState,
    owner: &User,
    invite: &PendingShareInviteView,
    since: Option<chrono::DateTime<chrono::Utc>>,
) -> InviteEmail {
    let Some(since) = since else {
        return InviteEmail::default();
    };
    let limits = invite_email_limit(owner.id);
    let over_limit = state.invite_limiter.check(&limits).is_err();
    let result = match &invite.claim_token {
        Some(token) => email_pending_invite(state, owner, invite, token, over_limit).await,
        None => report_account_invite(state, owner, invite, since, over_limit).await,
    };
    match result {
        Ok(email) => {
            if email.sent {
                state.invite_limiter.record(&limits);
            }
            email
        }
        Err(e) => {
            tracing::warn!(invite_id = %invite.id, error = %e, "invite email failed");
            InviteEmail::default()
        }
    }
}

async fn email_pending_invite(
    state: &AppState,
    owner: &User,
    invite: &PendingShareInviteView,
    token: &str,
    over_limit: bool,
) -> linkdean_core::Result<InviteEmail> {
    if let Some(account) = db::find_user_by_email(&state.pool, &invite.invitee_email).await? {
        let prefs = db::get_notification_email_prefs(&state.pool, account.id).await?;
        let wants = prefs
            .iter()
            .find(|(k, _)| *k == NotificationKind::ShareInvite)
            .is_none_or(|(_, on)| *on);
        if !wants {
            return Ok(InviteEmail::default());
        }
    }
    if over_limit {
        return Ok(InviteEmail {
            sent: false,
            limited: true,
        });
    }
    let (noun, name) = subject_name(&state.pool, owner.id, invite).await?;
    // The email carries its own link, not the owner's: opening it proves control of the inbox
    // and so also verifies the address, which the owner's copyable link must not do.
    let Some(email_token) = db::issue_email_claim_token(&state.pool, invite.id, token).await?
    else {
        return Ok(InviteEmail::default());
    };
    let data = json!({
        "owner": one_line(&owner_label(owner)),
        "subject_kind": noun,
        "name": name,
        "access": db::access_label(invite.access),
        "claim_url": claim_url(app_url(state), &email_token),
    });
    let queued = enqueue(state, &invite.invitee_email, "share_invite_pending", &data).await?;
    Ok(InviteEmail {
        sent: queued == Queued::Queued,
        limited: false,
    })
}

async fn report_account_invite(
    state: &AppState,
    owner: &User,
    invite: &PendingShareInviteView,
    since: chrono::DateTime<chrono::Utc>,
    over_limit: bool,
) -> linkdean_core::Result<InviteEmail> {
    let subject_type = match invite.subject_type {
        PendingShareSubjectType::Category => NotificationSubjectType::Category,
        PendingShareSubjectType::Dashboard => NotificationSubjectType::Dashboard,
    };
    let Some(id) = db::find_new_share_invite_email(
        &state.pool,
        owner.id,
        subject_type,
        invite.subject_id,
        since,
    )
    .await?
    else {
        return Ok(InviteEmail::default());
    };
    // The mailer may claim it between the insert and this check; then it goes out regardless.
    if over_limit && db::cancel_notification_email(&state.pool, id).await? {
        return Ok(InviteEmail {
            sent: false,
            limited: true,
        });
    }
    Ok(InviteEmail {
        sent: true,
        limited: false,
    })
}

async fn subject_name(
    pool: &PgPool,
    owner_id: Uuid,
    invite: &PendingShareInviteView,
) -> linkdean_core::Result<(&'static str, String)> {
    Ok(match invite.subject_type {
        PendingShareSubjectType::Category => (
            "folder",
            db::require_category_owner(pool, owner_id, invite.subject_id)
                .await?
                .name,
        ),
        PendingShareSubjectType::Dashboard => (
            "dashboard",
            db::require_dashboard_owner(pool, owner_id, invite.subject_id)
                .await?
                .title,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn pending(
        kind: NotificationKind,
        subject_type: NotificationSubjectType,
    ) -> PendingNotificationEmail {
        PendingNotificationEmail {
            id: Uuid::nil(),
            recipient_user_id: Uuid::nil(),
            recipient_email: "to@example.com".into(),
            actor_user_id: Some(Uuid::nil()),
            actor_label: Some("Ada\nLovelace".into()),
            kind,
            subject_type,
            subject_id: Uuid::nil(),
            subject_title: "Reading <b>list</b>".into(),
            body: "Ada did something".into(),
            payload: json!({ "access": "write" }),
            created_at: Utc::now(),
        }
    }

    const APP: &str = "https://links.example.com";

    #[test]
    fn a_share_invite_uses_the_invite_template() {
        let n = pending(
            NotificationKind::ShareInvite,
            NotificationSubjectType::Category,
        );
        let (template, data) = email_for(&n, APP);
        assert_eq!(template, "share_invite");
        assert_eq!(data["owner"], "Ada Lovelace");
        assert_eq!(data["subject_kind"], "folder");
        assert_eq!(data["access"], "read-write");
        assert_eq!(data["url"], "https://links.example.com/shared");
        assert_eq!(data["kind_label"], "Share invites");
        assert_eq!(
            data["settings_url"],
            "https://links.example.com/settings?tab=notifications"
        );
        let site = linkdean_mail::Site::new(APP);
        let r = linkdean_mail::render(template, &data, &site).unwrap();
        assert!(!r.subject.contains('\n'));
        assert!(!r.html.contains("<b>list"));
    }

    #[test]
    fn other_kinds_use_the_generic_template_and_link_to_their_page() {
        let n = pending(
            NotificationKind::SocialComment,
            NotificationSubjectType::Bookmark,
        );
        let (template, data) = email_for(&n, APP);
        assert_eq!(template, "notification");
        assert_eq!(data["heading"], "New comment");
        assert_eq!(data["message"], "Ada did something");
        assert_eq!(data["url"], "https://links.example.com/library");
        assert_eq!(data["kind_label"], "Comments");
        let site = linkdean_mail::Site::new(APP);
        linkdean_mail::render(template, &data, &site).unwrap();

        let paths = [
            (
                NotificationKind::ModerationChanges,
                NotificationSubjectType::Bookmark,
                "/trash".to_string(),
            ),
            (
                NotificationKind::ShareRevoked,
                NotificationSubjectType::Dashboard,
                "/shared".to_string(),
            ),
            (
                NotificationKind::SharedEdit,
                NotificationSubjectType::Category,
                "/shared".to_string(),
            ),
            (
                NotificationKind::SharedEdit,
                NotificationSubjectType::Dashboard,
                format!("/?board={}", Uuid::nil()),
            ),
            (
                NotificationKind::SocialLike,
                NotificationSubjectType::Category,
                format!("/library?folder={}", Uuid::nil()),
            ),
            (
                NotificationKind::UserFollowed,
                NotificationSubjectType::User,
                format!("/u/{}", Uuid::nil()),
            ),
        ];
        for (kind, subject_type, path) in paths {
            assert_eq!(path_for(&pending(kind, subject_type)), path, "{kind:?}");
        }
    }

    #[test]
    fn every_kind_renders() {
        let site = linkdean_mail::Site::new(APP);
        for kind in NotificationKind::ALL {
            let (template, data) =
                email_for(&pending(kind, NotificationSubjectType::Dashboard), APP);
            linkdean_mail::render(template, &data, &site).unwrap();
        }
    }

    #[test]
    fn the_claim_link_matches_the_web_app() {
        assert_eq!(
            claim_url("https://links.example.com/", "ab+c"),
            "https://links.example.com/register?share=ab%2Bc"
        );
    }

    #[test]
    fn the_share_response_gains_email_fields() {
        let v = with_invite_email(
            &json!({ "status": "pending" }),
            InviteEmail {
                sent: true,
                limited: false,
            },
        );
        assert_eq!(v, json!({ "status": "pending", "email_sent": true }));
        let v = with_invite_email(
            &json!({ "status": "pending" }),
            InviteEmail {
                sent: false,
                limited: true,
            },
        );
        assert_eq!(
            v,
            json!({ "status": "pending", "email_sent": false, "email_limited": true })
        );
    }
}
