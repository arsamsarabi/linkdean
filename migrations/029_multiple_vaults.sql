-- One user can hold many vaults. Each has its own passphrase and data key.

ALTER TABLE vaults DROP CONSTRAINT vaults_pkey;

ALTER TABLE vaults
    ADD COLUMN id UUID NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    ADD COLUMN name TEXT NOT NULL DEFAULT 'Vault',
    ADD COLUMN recovery_salt BYTEA,
    ADD COLUMN recovery_wrapped_key BYTEA,
    ADD COLUMN recovery_wrapped_nonce BYTEA,
    ADD COLUMN global_wrapped_key BYTEA,
    ADD COLUMN global_wrapped_nonce BYTEA;

ALTER TABLE vaults ALTER COLUMN name DROP DEFAULT;

CREATE UNIQUE INDEX vaults_user_name_uidx ON vaults (user_id, lower(name));

CREATE TABLE vault_master_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    kdf_salt BYTEA NOT NULL,
    wrapped_key BYTEA NOT NULL,
    wrapped_nonce BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE categories ADD COLUMN vault_id UUID REFERENCES vaults(id) ON DELETE CASCADE;
ALTER TABLE bookmarks ADD COLUMN vault_id UUID REFERENCES vaults(id) ON DELETE CASCADE;

UPDATE categories c SET vault_id = v.id
FROM vaults v
WHERE c.is_vault AND v.user_id = c.owner_user_id;

UPDATE bookmarks b SET vault_id = v.id
FROM vaults v
WHERE b.is_vault AND v.user_id = b.owner_user_id;

-- A vault category without a vault could hold nothing, so it becomes a plain one.
UPDATE categories SET is_vault = FALSE WHERE is_vault AND vault_id IS NULL;

ALTER TABLE categories ADD CONSTRAINT categories_vault_consistent
    CHECK (is_vault = (vault_id IS NOT NULL));
ALTER TABLE bookmarks ADD CONSTRAINT bookmarks_vault_consistent
    CHECK (is_vault = (vault_id IS NOT NULL));

CREATE INDEX categories_vault_idx ON categories (vault_id) WHERE vault_id IS NOT NULL;
CREATE INDEX bookmarks_vault_idx ON bookmarks (vault_id) WHERE vault_id IS NOT NULL;
