-- Broken-link checker: persist last probe result on non-vault bookmarks.

ALTER TABLE bookmarks
    ADD COLUMN link_check_status TEXT
        CHECK (link_check_status IS NULL OR link_check_status IN ('ok', 'broken', 'unreachable', 'unverifiable')),
    ADD COLUMN link_check_status_code INT,
    ADD COLUMN link_check_error TEXT,
    ADD COLUMN link_checked_at TIMESTAMPTZ,
    ADD COLUMN link_check_fail_count INT NOT NULL DEFAULT 0;

-- Worker claim: never-checked first, then oldest stale checks.
CREATE INDEX bookmarks_link_check_due_idx
    ON bookmarks (link_checked_at NULLS FIRST, created_at ASC)
    WHERE is_vault = FALSE AND deleted_at IS NULL AND url <> '';

CREATE INDEX bookmarks_link_check_broken_idx
    ON bookmarks (owner_user_id, link_checked_at DESC)
    WHERE is_vault = FALSE
      AND deleted_at IS NULL
      AND link_check_status IN ('broken', 'unreachable');
