-- Account bans and the audit trail for moderation.

CREATE TABLE user_bans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    ends_at TIMESTAMPTZ,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    lifted_at TIMESTAMPTZ,
    lifted_by UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX user_bans_open_idx ON user_bans (user_id) WHERE lifted_at IS NULL;

CREATE TABLE moderation_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    admin_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL CHECK (action IN (
        'ban',
        'ban_changed',
        'unban',
        'public_link_disabled',
        'category_private',
        'profile_hidden',
        'share_revoked',
        'comment_deleted',
        'bookmark_private',
        'removed_to_trash'
    )),
    subject_type TEXT,
    subject_id UUID,
    subject_title TEXT NOT NULL DEFAULT '',
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX moderation_actions_user_idx ON moderation_actions (user_id, created_at DESC);

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
        'user_followed',
        'moderation_changes'
    )
);
