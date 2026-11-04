-- A second claim link, sent only in the invite email. Claiming with it proves control of the
-- invited inbox, so it also verifies the address. The owner's copyable link never does.
ALTER TABLE pending_share_invites ADD COLUMN email_claim_token_hash TEXT;

CREATE UNIQUE INDEX pending_share_invites_email_claim_token
    ON pending_share_invites (email_claim_token_hash)
    WHERE email_claim_token_hash IS NOT NULL;
