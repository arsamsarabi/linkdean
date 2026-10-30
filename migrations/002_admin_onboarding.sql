ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS onboarding_completed BOOLEAN NOT NULL DEFAULT FALSE;

-- Existing first account → admin + onboarding done (already used the app)
UPDATE users
SET is_admin = TRUE,
    onboarding_completed = TRUE
WHERE id = (
    SELECT id FROM users ORDER BY created_at ASC LIMIT 1
);
