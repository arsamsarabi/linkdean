-- Personal activity history: per-user create/edit/delete/restore log.
-- Distinct from shared-activity notifications.

CREATE TABLE IF NOT EXISTS activity_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    subject_title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT activity_events_action_check CHECK (
        action IN ('created', 'updated', 'deleted', 'restored')
    ),
    CONSTRAINT activity_events_subject_type_check CHECK (
        subject_type IN ('bookmark', 'category', 'dashboard')
    )
);

CREATE INDEX IF NOT EXISTS activity_events_user_created_idx
    ON activity_events (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS activity_events_user_subject_idx
    ON activity_events (user_id, subject_type, created_at DESC);

CREATE INDEX IF NOT EXISTS activity_events_user_action_idx
    ON activity_events (user_id, action, created_at DESC);
