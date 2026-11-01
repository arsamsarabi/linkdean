-- Invites now cover existing accounts too. Access starts when the invitee accepts.

ALTER TABLE pending_share_invites
    ADD COLUMN invitee_user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    ADD COLUMN rejected_at TIMESTAMPTZ;

UPDATE pending_share_invites p
SET invitee_user_id = u.id
FROM users u
WHERE lower(u.email) = p.invitee_email
  AND p.claimed_at IS NULL
  AND p.revoked_at IS NULL;

CREATE INDEX pending_share_invites_invitee_open_idx
    ON pending_share_invites (invitee_user_id)
    WHERE claimed_at IS NULL AND revoked_at IS NULL;
