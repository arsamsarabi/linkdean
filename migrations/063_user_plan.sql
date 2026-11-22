-- Managed Free/Pro plan. NULL = self-host (platform quotas only).
ALTER TABLE users
    ADD COLUMN plan TEXT
    CHECK (plan IS NULL OR plan IN ('free', 'pro'));
