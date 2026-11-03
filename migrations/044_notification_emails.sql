-- Per-kind email preference; a missing row means the kind's default (NotificationKind::email_by_default).
CREATE TABLE user_notification_email_prefs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (user_id, kind)
);

-- Whether a notification should also be emailed: none, pending (waiting for the mailer), queued
-- (handed to the outbox). Rows made before this migration were never meant to be emailed.
ALTER TABLE notifications ADD COLUMN email_state TEXT NOT NULL DEFAULT 'none'
    CHECK (email_state IN ('none', 'pending', 'queued'));
CREATE INDEX notifications_email_pending ON notifications (created_at) WHERE email_state = 'pending';
