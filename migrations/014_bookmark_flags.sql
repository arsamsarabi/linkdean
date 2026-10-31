-- Per-user reading-list / favorite / pin flags (works on owned + shared bookmarks).

CREATE TABLE bookmark_flags (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bookmark_id UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    is_favorite BOOLEAN NOT NULL DEFAULT FALSE,
    is_unread BOOLEAN NOT NULL DEFAULT FALSE,
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, bookmark_id)
);

CREATE INDEX bookmark_flags_user_favorite_idx
    ON bookmark_flags (user_id)
    WHERE is_favorite = TRUE;

CREATE INDEX bookmark_flags_user_unread_idx
    ON bookmark_flags (user_id)
    WHERE is_unread = TRUE;

CREATE INDEX bookmark_flags_user_pinned_idx
    ON bookmark_flags (user_id)
    WHERE is_pinned = TRUE;
