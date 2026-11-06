-- Self-serve account deletion: the account is purged once this time passes.
ALTER TABLE users ADD COLUMN deletion_scheduled_at TIMESTAMPTZ;
CREATE INDEX users_deletion_scheduled_at_idx ON users (deletion_scheduled_at)
  WHERE deletion_scheduled_at IS NOT NULL;
