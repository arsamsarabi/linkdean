-- When the person last confirmed who they are in this session, for steps that need a fresh
-- sign-in (deleting a single sign-on only account). NULL until they do.
ALTER TABLE sessions ADD COLUMN reauthenticated_at TIMESTAMPTZ;
