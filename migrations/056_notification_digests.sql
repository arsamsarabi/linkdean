-- How a kind's email arrives: one email per notification, or collected into a daily digest.
ALTER TABLE user_notification_email_prefs ADD COLUMN delivery TEXT NOT NULL DEFAULT 'instant'
    CHECK (delivery IN ('instant', 'daily'));

-- 'digest': waiting for the recipient's next daily digest.
ALTER TABLE notifications DROP CONSTRAINT notifications_email_state_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_email_state_check
    CHECK (email_state IN ('none', 'pending', 'digest', 'queued'));
CREATE INDEX notifications_email_digest ON notifications (recipient_user_id, created_at)
    WHERE email_state = 'digest';

-- When each user's last digest went out, so it goes out at most once per local day.
CREATE TABLE notification_digest_runs (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    last_sent_at TIMESTAMPTZ NOT NULL
);

-- The one-click unsubscribe URL (RFC 8058) the sender puts in List-Unsubscribe, when there is one.
ALTER TABLE email_outbox ADD COLUMN list_unsubscribe TEXT;
