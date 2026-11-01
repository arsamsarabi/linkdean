-- Without outbound email, an address typed at registration proves nothing. An invite to
-- someone with no account is claimed with a single-use link the owner passes on.
ALTER TABLE pending_share_invites ADD COLUMN claim_token_hash TEXT;

CREATE UNIQUE INDEX pending_share_invites_claim_token
    ON pending_share_invites (claim_token_hash)
    WHERE claim_token_hash IS NOT NULL;
