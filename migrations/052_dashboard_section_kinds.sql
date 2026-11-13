-- Dashboard upgrades: a display mode per section, sections that fill themselves from the
-- owner's library, and per-person bookmark opens for "Most opened" and "Recently visited".

ALTER TABLE dashboard_sections
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'pinned'
        CHECK (kind IN ('pinned', 'latest', 'most_opened', 'recently_visited')),
    ADD COLUMN display TEXT NOT NULL DEFAULT 'medium'
        CHECK (display IN ('small', 'medium', 'large', 'icons')),
    ADD COLUMN item_limit INT NOT NULL DEFAULT 8
        CHECK (item_limit BETWEEN 4 AND 24),
    -- No foreign key: a deleted folder leaves the section empty instead of silently
    -- widening it to the whole library.
    ADD COLUMN source_category_id UUID,
    ADD COLUMN source_tag TEXT,
    ADD COLUMN source_site TEXT,
    ADD COLUMN source_favorites BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN source_unread BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN open_window TEXT NOT NULL DEFAULT 'all_time'
        CHECK (open_window IN ('last_30_days', 'all_time'));

-- One row per person and bookmark: the all-time count and the last open.
CREATE TABLE bookmark_open_stats (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bookmark_id UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    open_count BIGINT NOT NULL DEFAULT 0,
    last_opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, bookmark_id)
);

CREATE INDEX bookmark_open_stats_recent_idx ON bookmark_open_stats (user_id, last_opened_at DESC);

-- One row per open, kept for 30 days, for "Most opened" over the last 30 days.
CREATE TABLE bookmark_opens (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bookmark_id UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    opened_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX bookmark_opens_user_time_idx ON bookmark_opens (user_id, opened_at);
