-- Archived copies of bookmarks (roadmap item 12). One row per bookmark and format; the row is
-- both the queue entry and the record of the stored copy.
CREATE TABLE bookmark_archives (
    bookmark_id  UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    format       TEXT NOT NULL CHECK (format IN ('html', 'pdf', 'file')),
    status       TEXT NOT NULL CHECK (status IN ('queued', 'running', 'done', 'failed')),
    requested_by UUID REFERENCES users(id) ON DELETE SET NULL,
    -- File under ARCHIVE_DIR/{bookmark_id}/ holding the current copy; NULL until one exists.
    file_name    TEXT,
    bytes        BIGINT NOT NULL DEFAULT 0,
    content_type TEXT,
    -- NULL for vault bookmarks, so the address stays encrypted.
    final_url    TEXT,
    -- The vault whose key encrypted the file; NULL = stored in plain.
    key_vault_id UUID,
    nonce        BYTEA,
    captured_at  TIMESTAMPTZ,
    error        TEXT,
    queued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at   TIMESTAMPTZ,
    PRIMARY KEY (bookmark_id, format)
);
CREATE INDEX bookmark_archives_queued ON bookmark_archives (queued_at) WHERE status = 'queued';

ALTER TABLE platform_settings
    ADD COLUMN archives_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    -- Per-account archive storage; NULL = unlimited. The hosted beta sets 500 MB.
    ADD COLUMN max_archive_bytes BIGINT CHECK (max_archive_bytes IS NULL OR max_archive_bytes >= 0),
    ADD COLUMN max_archive_item_bytes BIGINT NOT NULL DEFAULT 26214400
        CHECK (max_archive_item_bytes BETWEEN 1048576 AND 1073741824);

ALTER TABLE user_settings
    ADD COLUMN archive_new BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN archive_formats TEXT[] NOT NULL DEFAULT ARRAY['html']
        CHECK (archive_formats <@ ARRAY['html', 'pdf'] AND cardinality(archive_formats) > 0);
