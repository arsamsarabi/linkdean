-- Milestone 1: share folders/lists with same-instance users (read / write).

CREATE TABLE category_shares (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    invitee_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access TEXT NOT NULL CHECK (access IN ('read', 'write')),
    invited_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (category_id, invitee_user_id),
    CHECK (invitee_user_id <> invited_by_user_id)
);

CREATE INDEX category_shares_invitee_idx ON category_shares (invitee_user_id);
CREATE INDEX category_shares_category_idx ON category_shares (category_id);
