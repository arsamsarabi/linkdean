-- Which Terms of Service version an account accepted at sign-up, and when. NULL for accounts
-- created before terms were recorded.
ALTER TABLE users ADD COLUMN terms_version TEXT;
ALTER TABLE users ADD COLUMN terms_accepted_at TIMESTAMPTZ;
