-- Email verification. Accounts that exist now count as verified.
ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;
UPDATE users SET email_verified_at = now();

-- Password reset tokens become account tokens with a purpose. One live token per user and
-- purpose; a new one voids the previous one.
ALTER TABLE password_reset_tokens RENAME TO account_tokens;
ALTER TABLE account_tokens ADD COLUMN purpose TEXT NOT NULL DEFAULT 'password_reset'
    CHECK (purpose IN ('password_reset', 'verify_email', 'change_email'));
ALTER TABLE account_tokens ALTER COLUMN purpose DROP DEFAULT;
-- The address an email change switches to. Set only for 'change_email'.
ALTER TABLE account_tokens ADD COLUMN new_email TEXT;
ALTER TABLE account_tokens ADD CONSTRAINT account_tokens_new_email
    CHECK ((purpose = 'change_email') = (new_email IS NOT NULL));
DROP INDEX password_reset_tokens_user;
CREATE INDEX account_tokens_user ON account_tokens (user_id, purpose) WHERE used_at IS NULL;

ALTER TABLE platform_settings ADD COLUMN require_verified_email BOOLEAN NOT NULL DEFAULT false;
