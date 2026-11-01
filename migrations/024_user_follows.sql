-- Follow graph between users (visibility / feed still backlog).

CREATE TABLE IF NOT EXISTS user_follows (
    follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (follower_id, following_id),
    CONSTRAINT user_follows_no_self CHECK (follower_id <> following_id)
);

CREATE INDEX IF NOT EXISTS user_follows_following_idx
    ON user_follows (following_id, created_at DESC);

CREATE INDEX IF NOT EXISTS user_follows_follower_idx
    ON user_follows (follower_id, created_at DESC);

ALTER TABLE notifications DROP CONSTRAINT IF EXISTS notifications_kind_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_kind_check CHECK (
    kind IN (
        'share_invite',
        'share_access_changed',
        'share_revoked',
        'shared_edit',
        'social_like',
        'social_comment',
        'social_rating',
        'user_followed'
    )
);

ALTER TABLE notifications DROP CONSTRAINT IF EXISTS notifications_subject_type_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_subject_type_check CHECK (
    subject_type IN ('category', 'dashboard', 'bookmark', 'user')
);
