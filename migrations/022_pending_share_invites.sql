-- Pending share invites for emails without an account yet.
-- Claimed on register / first OIDC signup matching invitee_email.

CREATE TABLE pending_share_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_type TEXT NOT NULL CHECK (subject_type IN ('category', 'dashboard')),
    subject_id UUID NOT NULL,
    invitee_email TEXT NOT NULL,
    access TEXT NOT NULL CHECK (access IN ('read', 'write')),
    invited_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    claimed_at TIMESTAMPTZ,
    claimed_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    revoked_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX pending_share_invites_active_uidx
    ON pending_share_invites (subject_type, subject_id, invitee_email)
    WHERE claimed_at IS NULL AND revoked_at IS NULL;

CREATE INDEX pending_share_invites_email_pending_idx
    ON pending_share_invites (invitee_email)
    WHERE claimed_at IS NULL AND revoked_at IS NULL;
