-- Milestone 5: likes, comments, ratings on shared bookmarks / lists / dashboards.

CREATE TABLE IF NOT EXISTS social_likes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, subject_type, subject_id),
    CONSTRAINT social_likes_subject_type_check CHECK (
        subject_type IN ('bookmark', 'category', 'dashboard')
    )
);

CREATE INDEX IF NOT EXISTS social_likes_subject_idx
    ON social_likes (subject_type, subject_id);

CREATE TABLE IF NOT EXISTS social_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT social_comments_subject_type_check CHECK (
        subject_type IN ('bookmark', 'category', 'dashboard')
    ),
    CONSTRAINT social_comments_body_len CHECK (
        char_length(body) BETWEEN 1 AND 4000
    )
);

CREATE INDEX IF NOT EXISTS social_comments_subject_created_idx
    ON social_comments (subject_type, subject_id, created_at ASC);

CREATE TABLE IF NOT EXISTS social_ratings (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    rating SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, subject_type, subject_id),
    CONSTRAINT social_ratings_subject_type_check CHECK (
        subject_type IN ('bookmark', 'category', 'dashboard')
    ),
    CONSTRAINT social_ratings_range CHECK (rating BETWEEN 1 AND 5)
);

CREATE INDEX IF NOT EXISTS social_ratings_subject_idx
    ON social_ratings (subject_type, subject_id);

ALTER TABLE notifications DROP CONSTRAINT IF EXISTS notifications_kind_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_kind_check CHECK (
    kind IN (
        'share_invite',
        'share_access_changed',
        'share_revoked',
        'shared_edit',
        'social_like',
        'social_comment',
        'social_rating'
    )
);
