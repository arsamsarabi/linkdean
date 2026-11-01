-- User block (hard cut) and mute (one-way quiet).

CREATE TABLE IF NOT EXISTS user_blocks (
    blocker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CONSTRAINT user_blocks_no_self CHECK (blocker_id <> blocked_id)
);

CREATE INDEX IF NOT EXISTS user_blocks_blocked_idx
    ON user_blocks (blocked_id, created_at DESC);

CREATE INDEX IF NOT EXISTS user_blocks_blocker_idx
    ON user_blocks (blocker_id, created_at DESC);

CREATE TABLE IF NOT EXISTS user_mutes (
    muter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    muted_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (muter_id, muted_id),
    CONSTRAINT user_mutes_no_self CHECK (muter_id <> muted_id)
);

CREATE INDEX IF NOT EXISTS user_mutes_muted_idx
    ON user_mutes (muted_id, created_at DESC);

CREATE INDEX IF NOT EXISTS user_mutes_muter_idx
    ON user_mutes (muter_id, created_at DESC);
