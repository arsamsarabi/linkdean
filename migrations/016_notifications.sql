-- In-app notifications for shared activity (invites, membership, shared edits).

CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipient_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    kind TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    subject_title TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT notifications_kind_check CHECK (
        kind IN (
            'share_invite',
            'share_access_changed',
            'share_revoked',
            'shared_edit'
        )
    ),
    CONSTRAINT notifications_subject_type_check CHECK (
        subject_type IN ('category', 'dashboard', 'bookmark')
    )
);

CREATE INDEX IF NOT EXISTS notifications_recipient_created_idx
    ON notifications (recipient_user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS notifications_recipient_unread_idx
    ON notifications (recipient_user_id, created_at DESC)
    WHERE read_at IS NULL;
