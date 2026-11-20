-- Named library filter snapshots (reopen as URL search params).
CREATE TABLE IF NOT EXISTS saved_searches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    filters JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT saved_searches_name_len CHECK (
        char_length(name) >= 1
        AND char_length(name) <= 80
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS saved_searches_owner_name_unique ON saved_searches (
    owner_user_id,
    lower(name)
);

CREATE INDEX IF NOT EXISTS saved_searches_owner_idx ON saved_searches (owner_user_id);
