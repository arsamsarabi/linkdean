# Multiple Vaults Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user keep several named vaults with their own passphrases, recovery keys and an optional global passphrase, and move categories and bookmarks into, out of and between them.

**Architecture:** `vaults` becomes many-per-user, and `categories` and `bookmarks` gain `vault_id`. A `VaultKeyring` of unlocked keys replaces the single `Option<&[u8; 32]>` through the repo layer. New repo modules hold vault lifecycle, the global passphrase and moves. The web app swaps its single vault panel for a vault list.

**Tech Stack:** Rust, axum, sqlx, Postgres 16, AES-GCM, Argon2id, React, Mantine, TanStack Query, zod, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-01-multiple-vaults-design.md`

---

## Conventions

Work on `main`. Commit each green task and push. End every commit message with:

```
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run `graphify update .` after each commit. Never commit `docs/IDEAS.md` changes you did not make.

Run DB tests against a throwaway container, never the dev database:

```bash
docker run -d --name linkdean-pg-check -e POSTGRES_USER=linkdean -e POSTGRES_PASSWORD=linkdean -e POSTGRES_DB=linkdean -p 127.0.0.1:55433:5432 --tmpfs /var/lib/postgresql/data postgres:16-alpine
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
export DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:55433/linkdean
```

Rust gate, before each Rust commit:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -cE "^crates.*(warning|error)"   # expect 0
cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result: FAILED|^error"                         # expect no output
```

Web gate, before each web commit:

```bash
cd apps/web && pnpm exec prettier --write src >/dev/null && pnpm exec tsc -b && pnpm lint && pnpm test
```

In scripts, check each step's exit code before committing. `set -e` does not stop on a failed heredoc in this shell, and that has already produced one bad commit.

## File structure

| File | Responsibility |
| --- | --- |
| `migrations/029_multiple_vaults.sql` | Schema and backfill |
| `crates/core/src/crypto/mod.rs` | Key wrapping and recovery key helpers |
| `crates/core/src/models/mod.rs` | `VaultKeyring`, `Vault` columns, request and view types |
| `crates/db/src/repo/vault.rs` | Vault lifecycle, recovery, vault search |
| `crates/db/src/repo/vault_global.rs` | Global passphrase |
| `crates/db/src/repo/vault_moves.rs` | Moves and blockers |
| `crates/api/src/routes/vault/mod.rs` | All `/vaults` routes |
| `crates/db/tests/vaults.rs`, `vault_global.rs`, `vault_moves.rs` | DB tests |
| `apps/web/src/schemas/vault/`, `hooks/vault/` | Web data layer |
| `apps/web/src/components/vault/` | Vault list, forms, recovery key, global passphrase, move modal |

---

### Task 1: Key wrapping helpers

**Files:** Modify `crates/core/src/crypto/mod.rs`.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` module of `crypto/mod.rs`, creating the module if the file has none:

```rust
#[cfg(test)]
mod key_wrap_tests {
    use super::*;

    #[test]
    fn a_secret_wrap_round_trips() {
        let key = [7u8; KEY_LEN];
        let wrapped = wrap_data_key("correct horse", &key).unwrap();
        let got = unlock_vault(
            "correct horse",
            &wrapped.kdf_salt,
            &wrapped.wrapped_key,
            &wrapped.wrapped_nonce,
        )
        .unwrap();
        assert_eq!(got, key);
    }

    #[test]
    fn a_wrong_secret_fails() {
        let wrapped = wrap_data_key("correct horse", &[7u8; KEY_LEN]).unwrap();
        let got = unlock_vault(
            "wrong horse",
            &wrapped.kdf_salt,
            &wrapped.wrapped_key,
            &wrapped.wrapped_nonce,
        );
        assert!(matches!(got, Err(Error::VaultUnlockFailed)));
    }

    #[test]
    fn a_master_key_wrap_round_trips() {
        let master = [1u8; KEY_LEN];
        let key = [2u8; KEY_LEN];
        let (ct, nonce) = wrap_with_key(&master, &key).unwrap();
        assert_eq!(unwrap_with_key(&master, &ct, &nonce).unwrap(), key);
        assert!(matches!(
            unwrap_with_key(&[3u8; KEY_LEN], &ct, &nonce),
            Err(Error::VaultUnlockFailed)
        ));
    }

    #[test]
    fn recovery_keys_are_long_and_normalize() {
        let code = new_recovery_key();
        assert_eq!(code.replace('-', "").len(), 52);
        assert_ne!(code, new_recovery_key());
        let messy = format!("  {} ", code.to_ascii_lowercase().replace('-', " "));
        assert_eq!(normalize_recovery_key(&messy), normalize_recovery_key(&code));
    }
}
```

Run: `cargo test -p linkdean-core key_wrap_tests`
Expected: compile error, `wrap_data_key` not found.

- [ ] **Step 2: Implement**

Add after `unlock_vault`:

```rust
/// A data key wrapped by a key derived from a secret.
pub struct WrappedKey {
    pub kdf_salt: Vec<u8>,
    pub wrapped_key: Vec<u8>,
    pub wrapped_nonce: Vec<u8>,
}

pub fn new_data_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Wrap `data_key` under `secret`. `unlock_vault` reverses it.
pub fn wrap_data_key(secret: &str, data_key: &[u8; KEY_LEN]) -> Result<WrappedKey> {
    let kdf_salt = random_bytes(SALT_LEN);
    let wrapping_key = derive_key(secret, &kdf_salt)?;
    let (wrapped_key, wrapped_nonce) = encrypt(&wrapping_key, data_key)?;
    Ok(WrappedKey {
        kdf_salt,
        wrapped_key,
        wrapped_nonce,
    })
}

/// Wrap a vault's data key under the user's master key.
pub fn wrap_with_key(
    master: &[u8; KEY_LEN],
    data_key: &[u8; KEY_LEN],
) -> Result<(Vec<u8>, Vec<u8>)> {
    encrypt(master, data_key)
}

pub fn unwrap_with_key(
    master: &[u8; KEY_LEN],
    wrapped_key: &[u8],
    wrapped_nonce: &[u8],
) -> Result<[u8; KEY_LEN]> {
    let plain =
        decrypt(master, wrapped_key, wrapped_nonce).map_err(|_| Error::VaultUnlockFailed)?;
    plain.try_into().map_err(|_| Error::VaultUnlockFailed)
}

const RECOVERY_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const RECOVERY_LEN: usize = 52;

/// 52 base32 characters (260 bits), grouped in fours for reading.
pub fn new_recovery_key() -> String {
    random_bytes(RECOVERY_LEN)
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .map(|b| RECOVERY_ALPHABET[(*b & 31) as usize] as char)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// The form a recovery key is wrapped under, so case and spacing do not matter.
pub fn normalize_recovery_key(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}
```

Replace the body of `setup_vault` so it uses them:

```rust
pub fn setup_vault(passphrase: &str) -> Result<VaultKeys> {
    let data_key = new_data_key();
    let wrapped = wrap_data_key(passphrase, &data_key)?;
    Ok(VaultKeys {
        kdf_salt: wrapped.kdf_salt,
        data_key,
        wrapped_key: wrapped.wrapped_key,
        wrapped_nonce: wrapped.wrapped_nonce,
    })
}
```

- [ ] **Step 3: Verify and commit**

Run: `cargo test -p linkdean-core key_wrap_tests`
Expected: 4 passed.

Commit: `Add key wrapping and recovery key helpers.`

---

### Task 2: Migration and models

**Files:** Create `migrations/029_multiple_vaults.sql`. Modify `crates/core/src/models/mod.rs`, `crates/db/src/repo/vault.rs`.

- [ ] **Step 1: Write the migration**

```sql
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
```

- [ ] **Step 2: Update the models**

In `crates/core/src/models/mod.rs`, replace `struct Vault` fields with:

```rust
pub struct Vault {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub kdf_salt: Vec<u8>,
    pub wrapped_key: Vec<u8>,
    pub wrapped_nonce: Vec<u8>,
    pub recovery_salt: Option<Vec<u8>>,
    pub recovery_wrapped_key: Option<Vec<u8>>,
    pub recovery_wrapped_nonce: Option<Vec<u8>>,
    pub global_wrapped_key: Option<Vec<u8>>,
    pub global_wrapped_nonce: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Add `pub vault_id: Option<Uuid>,` to `Category`, directly after `is_vault`, and to `Bookmark`, directly after `is_vault`. Add `pub vault_id: Option<Uuid>,` after `is_vault` in `CategoryView` and `BookmarkView`, and fill it from the row where each view is built: `vault_id: cat.vault_id` at `CategoryView::from_category`, `vault_id: b.vault_id` at both `BookmarkView` constructions in `bookmark_to_view`.

Add below `VaultPayload`:

```rust
/// Keys of the vaults a user has unlocked right now.
#[derive(Debug, Clone, Default)]
pub struct VaultKeyring(std::collections::HashMap<Uuid, [u8; 32]>);

impl VaultKeyring {
    pub fn insert(&mut self, vault_id: Uuid, key: [u8; 32]) {
        self.0.insert(vault_id, key);
    }

    pub fn get(&self, vault_id: Uuid) -> Option<&[u8; 32]> {
        self.0.get(&vault_id)
    }

    pub fn contains(&self, vault_id: Uuid) -> bool {
        self.0.contains_key(&vault_id)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn vault_ids(&self) -> Vec<Uuid> {
        self.0.keys().copied().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultView {
    pub id: Uuid,
    pub name: String,
    pub unlocked: bool,
    pub has_recovery_key: bool,
    pub in_global: bool,
    /// Only for unlocked vaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVaultRequest {
    pub name: String,
    pub passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameVaultRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverVaultRequest {
    pub recovery_key: String,
    pub new_passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVaultRequest {
    pub name: String,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeGlobalPassphraseRequest {
    pub current_passphrase: String,
    pub new_passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveToVaultRequest {
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub bookmark_ids: Vec<Uuid>,
    pub destination_vault_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultMoveBlockerKind {
    Shared,
    PublicLink,
    OtherCategory,
    Dashboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMoveBlocker {
    pub kind: VaultMoveBlockerKind,
    pub subject_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum VaultMoveOutcome {
    Moved {
        left_search: Vec<Uuid>,
        joined_search: Vec<Uuid>,
    },
    Blocked(Vec<VaultMoveBlocker>),
}
```

`repo/vault.rs` `setup_vault` needs no change: `name` is `NOT NULL` with no default now, so add it to the insert:

```rust
        INSERT INTO vaults (user_id, name, kdf_salt, wrapped_key, wrapped_nonce)
        VALUES ($1, 'Vault', $2, $3, $4)
```

`get_vault` must stop assuming one row. Change its query to `SELECT * FROM vaults WHERE user_id = $1 ORDER BY created_at LIMIT 1`.

- [ ] **Step 3: Verify and commit**

Run the Rust gate. Expected: green, including the migration on a fresh database.

Commit: `Add the multiple vaults schema.`

---

### Task 3: Thread the keyring through the repo layer

Mechanical. No behaviour changes yet: the API still has one vault per user.

**Files:** Modify `crates/api/src/state/mod.rs`, `crates/db/src/repo/{bookmarks,dashboards,categories,vault}.rs`, `crates/api/src/routes/{bookmarks,dashboards,categories,search,trash,auth,vault}/mod.rs`, every test that passes `None` as a vault key.

- [ ] **Step 1: Change `AppState`**

```rust
    /// user_id → unlocked vaults (session memory; lost on restart)
    pub vault_keys: Arc<RwLock<HashMap<Uuid, VaultKeyring>>>,
```

Replace the three methods:

```rust
    pub async fn vault_keyring(&self, user_id: Uuid) -> VaultKeyring {
        self.vault_keys
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn set_vault_key(&self, user_id: Uuid, vault_id: Uuid, key: [u8; 32]) {
        self.vault_keys
            .write()
            .await
            .entry(user_id)
            .or_default()
            .insert(vault_id, key);
    }

    pub async fn clear_vault_key(&self, user_id: Uuid, vault_id: Uuid) {
        let mut keys = self.vault_keys.write().await;
        let Some(ring) = keys.get(&user_id) else {
            return;
        };
        let mut next = VaultKeyring::default();
        for id in ring.vault_ids() {
            if id != vault_id {
                if let Some(key) = ring.get(id) {
                    next.insert(id, *key);
                }
            }
        }
        keys.insert(user_id, next);
    }

    pub async fn clear_vault_keys(&self, user_id: Uuid) {
        self.vault_keys.write().await.remove(&user_id);
    }
```

- [ ] **Step 2: Change the repo signatures**

In `bookmarks.rs` and `dashboards.rs`, replace every parameter `vault_key: Option<&[u8; 32]>` with `keyring: &VaultKeyring`, and rename pass-through uses of `vault_key` to `keyring`. Import `VaultKeyring` from `linkdean_core`.

Then make these specific changes.

`bookmark_to_view`:

```rust
    if b.is_vault {
        let Some(key) = b.vault_id.and_then(|id| keyring.get(id)) else {
            return Ok(None);
        };
```

`list_bookmarks`:

```rust
    let include_vault = include_vault && !keyring.is_empty();
```

and the per-row block, so only the owner decrypts:

```rust
    let none = VaultKeyring::default();
    let mut out = Vec::new();
    for b in rows {
        let ring = if b.is_vault && b.owner_user_id == user_id {
            keyring
        } else {
            &none
        };
        if let Some(v) = bookmark_to_view(pool, b, ring, user_id).await? {
            out.push(v);
        }
    }
    Ok(out)
```

`dashboards.rs:229` has the same conditional; apply the same pattern with a `none` keyring declared before its loop.

`create_bookmark`, temporarily, until Task 7 replaces this block:

```rust
    let mut vault_id = None;
    for cid in &cat_ids {
        let (cat, access) = require_category_write(pool, user_id, *cid).await?;
        if cat.is_vault {
            if !access.is_owner() {
                return Err(Error::Forbidden);
            }
            vault_id = cat.vault_id;
        }
    }
```

and in the encrypted branch:

```rust
    let bookmark = if let Some(vault_id) = vault_id {
        let key = *keyring.get(vault_id).ok_or(Error::VaultLocked)?;
```

with the insert becoming:

```rust
            INSERT INTO bookmarks (
                owner_user_id, category_id, url, title, description, notes,
                is_vault, vault_id, vault_ciphertext, vault_nonce
            ) VALUES ($1,$2,'','','',NULL,TRUE,$3,$4,$5)
```

binding `vault_id` after `primary`. Remove the now-unused `let key = *key;`.

`update_bookmark`:

```rust
        let key = if existing.is_vault { keyring } else { &VaultKeyring::default() };
```

becomes a local `let none = VaultKeyring::default();` plus `let ring = if existing.is_vault { keyring } else { &none };`, and the mutation branch reads the key from the row:

```rust
    if existing.is_vault {
        let key = *existing
            .vault_id
            .and_then(|id| keyring.get(id))
            .ok_or(Error::VaultLocked)?;
```

`categories.rs`: replace the `include_vault: bool` parameter of `list_categories` and `list_owned_categories` with `unlocked_vaults: &[Uuid]`, and in both queries replace `AND ($3 OR is_vault = FALSE)` with `AND (is_vault = FALSE OR vault_id = ANY($3))`, binding the slice. Update the `list_categories` and `list_owned_categories` callers to pass `&[]` where they passed `false`, and `&keyring.vault_ids()` where they passed a real value.

`vault.rs`:

```rust
pub async fn search_vault_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
    keyring: &VaultKeyring,
) -> Result<Vec<BookmarkView>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        "SELECT * FROM bookmarks WHERE owner_user_id = $1 AND vault_id = ANY($2)",
    )
    .bind(user_id)
    .bind(keyring.vault_ids())
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
```

with `bookmark_to_view(pool, b, keyring, user_id)` in the loop.

- [ ] **Step 3: Change the API callers**

In every route, replace:

| Before | After |
| --- | --- |
| `let key = state.vault_key(user.id).await;` | `let keyring = state.vault_keyring(user.id).await;` |
| `key.as_ref()` | `&keyring` |
| `key.is_some()` | `!keyring.is_empty()` |
| `state.vault_key(user.id).await.is_some()` | `!state.vault_keyring(user.id).await.is_empty()` |

`routes/categories/mod.rs:60`:

```rust
    let keyring = state.vault_keyring(user.id).await;
    let unlocked = if q.include_vault { keyring.vault_ids() } else { Vec::new() };
```

passing `&unlocked` to `list_categories`.

`routes/search/mod.rs:86`: `let keyring = state.vault_keyring(user.id).await;`, and both `if let Some(key) = unlocked {` become `if !keyring.is_empty() {` calling `db::search_vault_bookmarks(&state.pool, user.id, &q.q, &keyring)`. The `always` branch's `else` stays.

`routes/vault/mod.rs`, until Task 6 replaces it:

```rust
    db::setup_vault(&state.pool, user.id, &req.passphrase).await?;
    let vault = db::get_vault(&state.pool, user.id).await?;
    let key = db::unlock_vault_key(&state.pool, user.id, &req.passphrase).await?;
    state.set_vault_key(user.id, vault.id, key).await;
```

`unlock` follows the same shape. `lock` calls `state.clear_vault_keys(user.id).await`.

- [ ] **Step 4: Update the tests**

Replace vault-key arguments in `crates/db/tests/` with an empty keyring:

```bash
python3 - <<'EOF'
import glob, re
for p in glob.glob('crates/db/tests/*.rs'):
    s = open(p).read()
    n = re.sub(r'(db::(?:create_bookmark|update_bookmark|create_dashboard|get_dashboard|get_default_dashboard|put_dashboard|put_default_dashboard|patch_dashboard|bookmark_to_view)\((?:[^;]*?))\bNone(,?\s*\)\s*\n?\s*\.await)', r'\1&VaultKeyring::default()\2', s)
    if n != s:
        open(p, 'w').write(n)
EOF
```

The regex replaces only the last `None` of each call. Then `cargo check --tests -p linkdean-db` and fix what remains by hand, including `list_bookmarks` calls, and add `VaultKeyring` to each file's `linkdean_core` import.

In `crates/db/tests/authz.rs:467`, `is_vault: true` now breaks the new check constraint. Add before it:

```rust
    db::setup_vault(&pool, user_a.id, "vault-passphrase").await.expect("vault");
```

and have `create_category` set `vault_id` from the user's vault, temporarily until Task 7:

```rust
    let vault_id = if req.is_vault {
        Some(super::vault::get_vault(pool, user_id).await?.id)
    } else {
        None
    };
```

adding `vault_id` to the insert as column 7, bound last.

- [ ] **Step 5: Verify and commit**

Run the Rust gate. Expected: green.

Commit: `Pass a vault keyring through the repo layer.`

---

### Task 4: Vault lifecycle and recovery

**Files:** Modify `crates/db/src/repo/vault.rs`, `crates/db/src/repo/users.rs`. Test `crates/db/tests/vaults.rs`.

- [ ] **Step 1: Write the failing tests**

`crates/db/tests/vaults.rs`:

```rust
//! Several vaults per user, with their own keys.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{Error, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping vault tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@vaults.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user")
}

#[tokio::test]
async fn two_vaults_have_separate_keys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (work, work_key, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("work");
    let (home, home_key, _) = db::create_vault(&pool, owner.id, "Home", "home-passphrase")
        .await
        .expect("home");
    assert_ne!(work_key, home_key);

    let unlocked = db::unlock_vault(&pool, owner.id, work.id, "work-passphrase")
        .await
        .expect("unlock");
    assert_eq!(unlocked, work_key);
    let wrong = db::unlock_vault(&pool, owner.id, home.id, "work-passphrase").await;
    assert!(matches!(wrong, Err(Error::VaultUnlockFailed)));
}

#[tokio::test]
async fn names_are_unique_per_user_ignoring_case() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let other = user(&pool, "other").await;
    db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("first");
    let dup = db::create_vault(&pool, owner.id, "work", "work-passphrase").await;
    assert!(matches!(dup, Err(Error::Conflict(_))));
    db::create_vault(&pool, other.id, "Work", "work-passphrase")
        .await
        .expect("another user");
}

#[tokio::test]
async fn another_users_vault_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let stranger = user(&pool, "stranger").await;
    let (vault, _, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");

    let got = db::unlock_vault(&pool, stranger.id, vault.id, "work-passphrase").await;
    assert!(matches!(got, Err(Error::NotFound)));
    let got = db::rename_vault(&pool, stranger.id, vault.id, "Mine").await;
    assert!(matches!(got, Err(Error::NotFound)));
}

#[tokio::test]
async fn recovery_key_sets_a_new_passphrase() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, recovery) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");

    let wrong = db::recover_vault(&pool, owner.id, vault.id, "NOPE", "new-passphrase").await;
    assert!(matches!(wrong, Err(Error::VaultUnlockFailed)));

    let messy = recovery.to_ascii_lowercase().replace('-', " ");
    let got = db::recover_vault(&pool, owner.id, vault.id, &messy, "new-passphrase")
        .await
        .expect("recover");
    assert_eq!(got, key);

    assert_eq!(
        db::unlock_vault(&pool, owner.id, vault.id, "new-passphrase")
            .await
            .expect("new"),
        key
    );
    let old = db::unlock_vault(&pool, owner.id, vault.id, "work-passphrase").await;
    assert!(matches!(old, Err(Error::VaultUnlockFailed)));
}

#[tokio::test]
async fn replacing_a_recovery_key_invalidates_the_old_one() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, old) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");

    let new = db::replace_recovery_key(&pool, owner.id, vault.id, &key)
        .await
        .expect("replace");
    let got = db::recover_vault(&pool, owner.id, vault.id, &old, "new-passphrase").await;
    assert!(matches!(got, Err(Error::VaultUnlockFailed)));
    db::recover_vault(&pool, owner.id, vault.id, &new, "new-passphrase")
        .await
        .expect("recover");
}
```

Run: `cargo test -p linkdean-db --test vaults`
Expected: compile error, `create_vault` not found.

- [ ] **Step 2: Implement**

Replace everything below `search_vault_bookmarks` in `crates/db/src/repo/vault.rs`:

```rust
fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(Error::BadRequest("vault name must be 1-60 characters".into()));
    }
    Ok(name.to_string())
}

fn map_name_conflict(e: sqlx::Error) -> Error {
    match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("vaults_user_name_uidx") => {
            Error::Conflict("You already have a vault with this name.".into())
        }
        _ => Error::msg(e.to_string()),
    }
}

/// Create a vault. Returns the row, its data key and its recovery key, shown once.
pub async fn create_vault(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    passphrase: &str,
) -> Result<(Vault, [u8; 32], String)> {
    let name = clean_name(name)?;
    let passphrase = passphrase.to_string();
    let (key, by_passphrase, recovery, by_recovery) = super::blocking::run_blocking(move || {
        let key = crypto::new_data_key();
        let by_passphrase = crypto::wrap_data_key(&passphrase, &key)?;
        let recovery = crypto::new_recovery_key();
        let by_recovery = crypto::wrap_data_key(&crypto::normalize_recovery_key(&recovery), &key)?;
        Ok((key, by_passphrase, recovery, by_recovery))
    })
    .await?;

    let vault = sqlx::query_as::<_, Vault>(
        r#"
        INSERT INTO vaults (
            user_id, name, kdf_salt, wrapped_key, wrapped_nonce,
            recovery_salt, recovery_wrapped_key, recovery_wrapped_nonce
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(&by_passphrase.kdf_salt)
    .bind(&by_passphrase.wrapped_key)
    .bind(&by_passphrase.wrapped_nonce)
    .bind(&by_recovery.kdf_salt)
    .bind(&by_recovery.wrapped_key)
    .bind(&by_recovery.wrapped_nonce)
    .fetch_one(pool)
    .await
    .map_err(map_name_conflict)?;
    Ok((vault, key, recovery))
}

pub async fn list_vaults(pool: &PgPool, user_id: Uuid) -> Result<Vec<Vault>> {
    sqlx::query_as::<_, Vault>("SELECT * FROM vaults WHERE user_id = $1 ORDER BY lower(name)")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_user_vault(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Vault> {
    sqlx::query_as::<_, Vault>("SELECT * FROM vaults WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

/// Active categories and bookmarks in a vault.
pub async fn vault_item_counts(pool: &PgPool, vault_id: Uuid) -> Result<(i64, i64)> {
    sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM categories WHERE vault_id = $1 AND deleted_at IS NULL),
            (SELECT COUNT(*) FROM bookmarks WHERE vault_id = $1 AND deleted_at IS NULL)
        "#,
    )
    .bind(vault_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn rename_vault(pool: &PgPool, user_id: Uuid, id: Uuid, name: &str) -> Result<Vault> {
    let name = clean_name(name)?;
    get_user_vault(pool, user_id, id).await?;
    sqlx::query_as::<_, Vault>(
        "UPDATE vaults SET name = $1, updated_at = now() WHERE id = $2 RETURNING *",
    )
    .bind(name)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(map_name_conflict)
}

pub async fn unlock_vault(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    passphrase: &str,
) -> Result<[u8; 32]> {
    let vault = get_user_vault(pool, user_id, id).await?;
    let passphrase = passphrase.to_string();
    super::blocking::run_blocking(move || {
        crypto::unlock_vault(
            &passphrase,
            &vault.kdf_salt,
            &vault.wrapped_key,
            &vault.wrapped_nonce,
        )
    })
    .await
}

/// Set a new passphrase using the recovery key. Returns the data key.
pub async fn recover_vault(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    recovery_key: &str,
    new_passphrase: &str,
) -> Result<[u8; 32]> {
    let vault = get_user_vault(pool, user_id, id).await?;
    let (Some(salt), Some(wrapped), Some(nonce)) = (
        vault.recovery_salt,
        vault.recovery_wrapped_key,
        vault.recovery_wrapped_nonce,
    ) else {
        return Err(Error::VaultUnlockFailed);
    };
    let recovery = crypto::normalize_recovery_key(recovery_key);
    let new_passphrase = new_passphrase.to_string();
    let (key, by_passphrase) = super::blocking::run_blocking(move || {
        let key = crypto::unlock_vault(&recovery, &salt, &wrapped, &nonce)?;
        let by_passphrase = crypto::wrap_data_key(&new_passphrase, &key)?;
        Ok((key, by_passphrase))
    })
    .await?;

    sqlx::query(
        r#"
        UPDATE vaults
        SET kdf_salt = $1, wrapped_key = $2, wrapped_nonce = $3, updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(&by_passphrase.kdf_salt)
    .bind(&by_passphrase.wrapped_key)
    .bind(&by_passphrase.wrapped_nonce)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(key)
}

/// Create or replace the recovery key. The caller holds the unlocked data key.
pub async fn replace_recovery_key(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    data_key: &[u8; 32],
) -> Result<String> {
    get_user_vault(pool, user_id, id).await?;
    let key = *data_key;
    let (recovery, wrapped) = super::blocking::run_blocking(move || {
        let recovery = crypto::new_recovery_key();
        let wrapped = crypto::wrap_data_key(&crypto::normalize_recovery_key(&recovery), &key)?;
        Ok((recovery, wrapped))
    })
    .await?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET recovery_salt = $1, recovery_wrapped_key = $2, recovery_wrapped_nonce = $3,
            updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(&wrapped.kdf_salt)
    .bind(&wrapped.wrapped_key)
    .bind(&wrapped.wrapped_nonce)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(recovery)
}

/// Permanently delete a vault with its categories and bookmarks.
pub async fn delete_vault(
    pool: &PgPool,
    user: &User,
    id: Uuid,
    typed_name: &str,
    password: Option<&str>,
) -> Result<()> {
    let vault = get_user_vault(pool, user.id, id).await?;
    if typed_name != vault.name {
        return Err(Error::BadRequest("The vault name doesn't match.".into()));
    }
    if let Some(hash) = user.password_hash.clone() {
        let password = password.unwrap_or_default().to_string();
        let ok = super::blocking::run_blocking(move || crypto::verify_password(&password, &hash))
            .await?;
        if !ok {
            return Err(Error::BadRequest("The account password is incorrect.".into()));
        }
    }
    sqlx::query("DELETE FROM vaults WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}
```

Keep `setup_vault`, `get_vault` and `unlock_vault_key` below these until Task 11. Rename nothing else. Update the imports: `use linkdean_core::{Bookmark, BookmarkView, Error, Result, User, Vault, VaultKeyring};`.

If `verify_password` errors on an invalid hash, treat that as the existing error; do not special-case it.

- [ ] **Step 3: Verify and commit**

Run: `cargo test -p linkdean-db --test vaults`
Expected: 5 passed.

Run the Rust gate.

Commit: `Add vault creation, recovery and deletion.`

---

### Task 5: Global passphrase

**Files:** Create `crates/db/src/repo/vault_global.rs`. Modify `crates/db/src/repo/mod.rs`. Test `crates/db/tests/vault_global.rs`.

- [ ] **Step 1: Write the failing tests**

`crates/db/tests/vault_global.rs`:

```rust
//! The optional global passphrase. It adds a way in and replaces nothing.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{Error, User, Vault};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping global passphrase tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("global-{}@vaults.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user")
}

async fn vault(pool: &PgPool, owner: &User, name: &str) -> (Vault, [u8; 32]) {
    let (vault, key, _) = db::create_vault(pool, owner.id, name, "vault-passphrase")
        .await
        .expect("vault");
    (vault, key)
}

#[tokio::test]
async fn unlocks_only_joined_vaults_and_own_passphrases_keep_working() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, joined_key) = vault(&pool, &owner, "Joined").await;
    let (apart, _) = vault(&pool, &owner, "Apart").await;

    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, joined.id, &joined_key, "global-passphrase")
        .await
        .expect("join");

    let opened = db::unlock_with_global(&pool, owner.id, "global-passphrase")
        .await
        .expect("unlock");
    assert_eq!(opened, vec![(joined.id, joined_key)]);
    assert!(opened.iter().all(|(id, _)| *id != apart.id));

    assert_eq!(
        db::unlock_vault(&pool, owner.id, joined.id, "vault-passphrase")
            .await
            .expect("own passphrase"),
        joined_key
    );
}

#[tokio::test]
async fn wrong_passphrase_fails() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key) = vault(&pool, &owner, "Joined").await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");

    let join = db::join_global(&pool, owner.id, joined.id, &key, "wrong-passphrase").await;
    assert!(matches!(join, Err(Error::VaultUnlockFailed)));
    let open = db::unlock_with_global(&pool, owner.id, "wrong-passphrase").await;
    assert!(matches!(open, Err(Error::VaultUnlockFailed)));
}

#[tokio::test]
async fn enabling_twice_conflicts() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    let again = db::enable_global_passphrase(&pool, owner.id, "other-passphrase").await;
    assert!(matches!(again, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn changing_keeps_vaults_joined() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key) = vault(&pool, &owner, "Joined").await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, joined.id, &key, "global-passphrase")
        .await
        .expect("join");

    db::change_global_passphrase(&pool, owner.id, "global-passphrase", "changed-passphrase")
        .await
        .expect("change");

    let old = db::unlock_with_global(&pool, owner.id, "global-passphrase").await;
    assert!(matches!(old, Err(Error::VaultUnlockFailed)));
    let opened = db::unlock_with_global(&pool, owner.id, "changed-passphrase")
        .await
        .expect("unlock");
    assert_eq!(opened, vec![(joined.id, key)]);
}

#[tokio::test]
async fn leaving_and_disabling_clear_the_wrapped_keys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (first, first_key) = vault(&pool, &owner, "First").await;
    let (second, second_key) = vault(&pool, &owner, "Second").await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, first.id, &first_key, "global-passphrase")
        .await
        .expect("join first");
    db::join_global(&pool, owner.id, second.id, &second_key, "global-passphrase")
        .await
        .expect("join second");

    db::leave_global(&pool, owner.id, first.id).await.expect("leave");
    let opened = db::unlock_with_global(&pool, owner.id, "global-passphrase")
        .await
        .expect("unlock");
    assert_eq!(opened, vec![(second.id, second_key)]);

    db::disable_global_passphrase(&pool, owner.id)
        .await
        .expect("disable");
    assert!(!db::has_global_passphrase(&pool, owner.id).await.expect("state"));
    assert!(db::list_vaults(&pool, owner.id)
        .await
        .expect("list")
        .iter()
        .all(|v| v.global_wrapped_key.is_none()));
    db::unlock_vault(&pool, owner.id, second.id, "vault-passphrase")
        .await
        .expect("own passphrase survives");
}

#[tokio::test]
async fn recovery_does_not_change_membership() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key, recovery) = db::create_vault(&pool, owner.id, "Joined", "vault-passphrase")
        .await
        .expect("vault");
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, joined.id, &key, "global-passphrase")
        .await
        .expect("join");

    db::recover_vault(&pool, owner.id, joined.id, &recovery, "new-passphrase")
        .await
        .expect("recover");
    assert_eq!(
        db::unlock_with_global(&pool, owner.id, "global-passphrase")
            .await
            .expect("unlock"),
        vec![(joined.id, key)]
    );
}
```

Run: `cargo test -p linkdean-db --test vault_global`
Expected: compile error, `enable_global_passphrase` not found.

- [ ] **Step 2: Implement**

`crates/db/src/repo/vault_global.rs`:

```rust
use linkdean_core::crypto;
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::vault::{get_user_vault, list_vaults};

#[derive(sqlx::FromRow)]
struct MasterKeyRow {
    kdf_salt: Vec<u8>,
    wrapped_key: Vec<u8>,
    wrapped_nonce: Vec<u8>,
}

async fn master_row(pool: &PgPool, user_id: Uuid) -> Result<Option<MasterKeyRow>> {
    sqlx::query_as::<_, MasterKeyRow>(
        "SELECT kdf_salt, wrapped_key, wrapped_nonce FROM vault_master_keys WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

async fn open_master(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<[u8; 32]> {
    let row = master_row(pool, user_id).await?.ok_or(Error::NotFound)?;
    let passphrase = passphrase.to_string();
    super::blocking::run_blocking(move || {
        crypto::unlock_vault(&passphrase, &row.kdf_salt, &row.wrapped_key, &row.wrapped_nonce)
    })
    .await
}

pub async fn has_global_passphrase(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    Ok(master_row(pool, user_id).await?.is_some())
}

pub async fn enable_global_passphrase(
    pool: &PgPool,
    user_id: Uuid,
    passphrase: &str,
) -> Result<()> {
    if has_global_passphrase(pool, user_id).await? {
        return Err(Error::Conflict("Global passphrase is already on.".into()));
    }
    let passphrase = passphrase.to_string();
    let wrapped = super::blocking::run_blocking(move || {
        crypto::wrap_data_key(&passphrase, &crypto::new_data_key())
    })
    .await?;
    sqlx::query(
        r#"
        INSERT INTO vault_master_keys (user_id, kdf_salt, wrapped_key, wrapped_nonce)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(&wrapped.kdf_salt)
    .bind(&wrapped.wrapped_key)
    .bind(&wrapped.wrapped_nonce)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Only the master key is rewrapped, so joined vaults stay joined.
pub async fn change_global_passphrase(
    pool: &PgPool,
    user_id: Uuid,
    current: &str,
    new_passphrase: &str,
) -> Result<()> {
    let master = open_master(pool, user_id, current).await?;
    let new_passphrase = new_passphrase.to_string();
    let wrapped =
        super::blocking::run_blocking(move || crypto::wrap_data_key(&new_passphrase, &master))
            .await?;
    sqlx::query(
        r#"
        UPDATE vault_master_keys
        SET kdf_salt = $1, wrapped_key = $2, wrapped_nonce = $3, updated_at = now()
        WHERE user_id = $4
        "#,
    )
    .bind(&wrapped.kdf_salt)
    .bind(&wrapped.wrapped_key)
    .bind(&wrapped.wrapped_nonce)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Removes the global path only. No vault contents or passphrases change.
pub async fn disable_global_passphrase(pool: &PgPool, user_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET global_wrapped_key = NULL, global_wrapped_nonce = NULL, updated_at = now()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query("DELETE FROM vault_master_keys WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))
}

/// The caller holds the vault's unlocked data key.
pub async fn join_global(
    pool: &PgPool,
    user_id: Uuid,
    vault_id: Uuid,
    data_key: &[u8; 32],
    passphrase: &str,
) -> Result<()> {
    get_user_vault(pool, user_id, vault_id).await?;
    let master = open_master(pool, user_id, passphrase).await?;
    let (ct, nonce) = crypto::wrap_with_key(&master, data_key)?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET global_wrapped_key = $1, global_wrapped_nonce = $2, updated_at = now()
        WHERE id = $3
        "#,
    )
    .bind(ct)
    .bind(nonce)
    .bind(vault_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn leave_global(pool: &PgPool, user_id: Uuid, vault_id: Uuid) -> Result<()> {
    get_user_vault(pool, user_id, vault_id).await?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET global_wrapped_key = NULL, global_wrapped_nonce = NULL, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(vault_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Keys of every joined vault.
pub async fn unlock_with_global(
    pool: &PgPool,
    user_id: Uuid,
    passphrase: &str,
) -> Result<Vec<(Uuid, [u8; 32])>> {
    let master = open_master(pool, user_id, passphrase).await?;
    let mut out = Vec::new();
    for vault in list_vaults(pool, user_id).await? {
        if let (Some(ct), Some(nonce)) = (vault.global_wrapped_key, vault.global_wrapped_nonce) {
            out.push((vault.id, crypto::unwrap_with_key(&master, &ct, &nonce)?));
        }
    }
    Ok(out)
}
```

In `crates/db/src/repo/mod.rs`, add `mod vault_global;` and `pub use vault_global::*;`, following how the file already exports `vault`.

- [ ] **Step 3: Verify and commit**

Run: `cargo test -p linkdean-db --test vault_global`
Expected: 6 passed.

Run the Rust gate.

Commit: `Add the optional global vault passphrase.`

---

### Task 6: Vault routes

The spec's route table gains `POST /vaults/lock-all`, needed by auto-lock. Add it to the spec's table in this task.

**Files:** Modify `crates/api/src/routes/vault/mod.rs`, `docs/superpowers/specs/2026-11-01-multiple-vaults-design.md`.

- [ ] **Step 1: Add the routes**

Keep the four `/vault/...` routes until Task 11, so the web app keeps working. Add to `router()`:

```rust
        .route("/vaults", get(list).post(create))
        .route("/vaults/lock-all", post(lock_all))
        .route(
            "/vaults/global",
            get(global_status)
                .post(global_enable)
                .put(global_change)
                .delete(global_disable),
        )
        .route("/vaults/global/unlock", post(global_unlock))
        .route("/vaults/{id}", patch(rename).delete(delete_one))
        .route("/vaults/{id}/unlock", post(unlock_one))
        .route("/vaults/{id}/lock", post(lock_one))
        .route("/vaults/{id}/recover", post(recover))
        .route("/vaults/{id}/recovery-key", post(new_recovery_key))
        .route("/vaults/{id}/global", post(global_join).delete(global_leave))
```

Handlers. Update the imports for `Path`, `patch` and the request types:

```rust
fn check_passphrase(passphrase: &str) -> Result<(), crate::error::ApiError> {
    if passphrase.len() < 8 {
        return Err(Error::BadRequest("vault passphrase min 8 chars".into()).into());
    }
    Ok(())
}

async fn to_view(state: &AppState, keyring: &VaultKeyring, vault: Vault) -> ApiResult<VaultView> {
    let unlocked = keyring.contains(vault.id);
    let counts = if unlocked {
        Some(db::vault_item_counts(&state.pool, vault.id).await?)
    } else {
        None
    };
    Ok(VaultView {
        id: vault.id,
        name: vault.name,
        unlocked,
        has_recovery_key: vault.recovery_wrapped_key.is_some(),
        in_global: vault.global_wrapped_key.is_some(),
        category_count: counts.map(|c| c.0),
        bookmark_count: counts.map(|c| c.1),
    })
}

async fn list(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(user.id).await;
    let mut vaults = Vec::new();
    for vault in db::list_vaults(&state.pool, user.id).await? {
        vaults.push(to_view(&state, &keyring, vault).await?);
    }
    Ok(Json(json!({ "vaults": vaults })))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateVaultRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.passphrase)?;
    let (vault, key, recovery_key) =
        db::create_vault(&state.pool, user.id, &req.name, &req.passphrase).await?;
    state.set_vault_key(user.id, vault.id, key).await;
    let keyring = state.vault_keyring(user.id).await;
    let view = to_view(&state, &keyring, vault).await?;
    Ok(Json(json!({ "vault": view, "recovery_key": recovery_key })))
}

async fn rename(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameVaultRequest>,
) -> ApiResult<Json<Value>> {
    let vault = db::rename_vault(&state.pool, user.id, id, &req.name).await?;
    let keyring = state.vault_keyring(user.id).await;
    let view = to_view(&state, &keyring, vault).await?;
    Ok(Json(json!({ "vault": view })))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<DeleteVaultRequest>,
) -> ApiResult<Json<Value>> {
    db::delete_vault(&state.pool, &user, id, &req.name, req.password.as_deref()).await?;
    state.clear_vault_key(user.id, id).await;
    Ok(Json(json!({ "ok": true })))
}

async fn unlock_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let key = db::unlock_vault(&state.pool, user.id, id, &req.passphrase).await?;
    state.set_vault_key(user.id, id, key).await;
    Ok(Json(json!({ "ok": true })))
}

async fn lock_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::get_user_vault(&state.pool, user.id, id).await?;
    state.clear_vault_key(user.id, id).await;
    Ok(Json(json!({ "ok": true })))
}

async fn lock_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    state.clear_vault_keys(user.id).await;
    Ok(Json(json!({ "ok": true })))
}

async fn recover(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RecoverVaultRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.new_passphrase)?;
    let key = db::recover_vault(
        &state.pool,
        user.id,
        id,
        &req.recovery_key,
        &req.new_passphrase,
    )
    .await?;
    state.set_vault_key(user.id, id, key).await;
    Ok(Json(json!({ "ok": true })))
}

async fn new_recovery_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(user.id).await;
    let key = keyring.get(id).ok_or(Error::VaultLocked)?;
    let recovery_key = db::replace_recovery_key(&state.pool, user.id, id, key).await?;
    Ok(Json(json!({ "recovery_key": recovery_key })))
}

async fn global_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let enabled = db::has_global_passphrase(&state.pool, user.id).await?;
    Ok(Json(json!({ "enabled": enabled })))
}

async fn global_enable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.passphrase)?;
    db::enable_global_passphrase(&state.pool, user.id, &req.passphrase).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn global_change(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<ChangeGlobalPassphraseRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.new_passphrase)?;
    db::change_global_passphrase(
        &state.pool,
        user.id,
        &req.current_passphrase,
        &req.new_passphrase,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn global_disable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    db::disable_global_passphrase(&state.pool, user.id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn global_unlock(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let opened = db::unlock_with_global(&state.pool, user.id, &req.passphrase).await?;
    let mut ids = Vec::with_capacity(opened.len());
    for (id, key) in opened {
        state.set_vault_key(user.id, id, key).await;
        ids.push(id);
    }
    Ok(Json(json!({ "unlocked": ids })))
}

async fn global_join(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(user.id).await;
    let key = keyring.get(id).ok_or(Error::VaultLocked)?;
    db::join_global(&state.pool, user.id, id, key, &req.passphrase).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn global_leave(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::leave_global(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
```

Add to the spec's API table: `| `POST /vaults/lock-all` | Lock every vault, used by auto-lock |`.

- [ ] **Step 2: Verify and commit**

Run the Rust gate.

Commit: `Add the vault routes.`

---

### Task 7: Create in a vault, and the web data layer

`CreateCategory.is_vault` becomes `vault_id`, so the web drawer changes in the same commit.

**Files:** Modify `crates/core/src/models/mod.rs`, `crates/db/src/repo/{categories,bookmarks}.rs`, `crates/api/src/routes/categories/mod.rs`, `apps/web/src/schemas/vault/vault.ts`, `apps/web/src/hooks/vault/useVault.ts`, `apps/web/src/api/{api,client}.ts`, `apps/web/src/components/category/CategoryDrawer.tsx`, `apps/web/src/components/bookmark/BookmarkDrawer.tsx`. Test `crates/db/tests/vaults.rs`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/db/tests/vaults.rs`, and change its import to `use linkdean_core::{CreateBookmark, CreateCategory, Error, User, VaultKeyring};`:

```rust
#[tokio::test]
async fn deleting_a_vault_removes_its_contents() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    let cat = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Secret".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: Some(vault.id),
            icon: None,
        },
    )
    .await
    .expect("category");
    let bookmark = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://secret.example/".into(),
            category_ids: vec![cat.id],
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("bookmark");

    let wrong = db::delete_vault(&pool, &owner, vault.id, "Nope", Some("account-password")).await;
    assert!(matches!(wrong, Err(Error::BadRequest(_))));
    let wrong = db::delete_vault(&pool, &owner, vault.id, "Work", Some("bad")).await;
    assert!(matches!(wrong, Err(Error::BadRequest(_))));

    db::delete_vault(&pool, &owner, vault.id, "Work", Some("account-password"))
        .await
        .expect("delete");
    assert!(db::get_bookmark_raw(&pool, owner.id, bookmark.id).await.is_err());
    assert!(db::list_vaults(&pool, owner.id).await.expect("list").is_empty());
}

async fn vault_category(pool: &PgPool, owner: &User, vault_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Cat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

#[tokio::test]
async fn a_locked_vaults_items_stay_hidden_while_another_is_open() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (open, open_key, _) = db::create_vault(&pool, owner.id, "Open", "open-passphrase")
        .await
        .expect("open");
    let (shut, shut_key, _) = db::create_vault(&pool, owner.id, "Shut", "shut-passphrase")
        .await
        .expect("shut");
    let mut both = VaultKeyring::default();
    both.insert(open.id, open_key);
    both.insert(shut.id, shut_key);

    let open_cat = vault_category(&pool, &owner, Some(open.id)).await;
    let shut_cat = vault_category(&pool, &owner, Some(shut.id)).await;
    for (cat, url) in [(open_cat, "https://open.example/"), (shut_cat, "https://shut.example/")] {
        db::create_bookmark(
            &pool,
            owner.id,
            CreateBookmark {
                url: url.into(),
                category_ids: vec![cat],
                ..Default::default()
            },
            &both,
        )
        .await
        .expect("bookmark");
    }

    let mut only_open = VaultKeyring::default();
    only_open.insert(open.id, open_key);
    let list = db::list_bookmarks(&pool, owner.id, None, true, &only_open, false)
        .await
        .expect("list");
    assert_eq!(
        list.iter().map(|b| b.url.as_str()).collect::<Vec<_>>(),
        vec!["https://open.example/"]
    );

    let cats = db::list_categories(&pool, owner.id, false, &only_open.vault_ids(), false)
        .await
        .expect("categories");
    assert!(cats.iter().any(|c| c.id == open_cat));
    assert!(cats.iter().all(|c| c.id != shut_cat));
}

#[tokio::test]
async fn a_loose_bookmark_lives_in_a_vault_without_a_category() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Loose", "loose-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);

    let view = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://loose.example/".into(),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("bookmark");
    assert!(view.is_vault);
    assert!(view.categories.is_empty());
    assert_eq!(view.url, "https://loose.example/");

    let locked = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://loose.example/2".into(),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(locked, Err(Error::VaultLocked)));
}

#[tokio::test]
async fn a_bookmark_cannot_span_vaults() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (first, first_key, _) = db::create_vault(&pool, owner.id, "First", "first-passphrase")
        .await
        .expect("first");
    let (second, second_key, _) = db::create_vault(&pool, owner.id, "Second", "second-passphrase")
        .await
        .expect("second");
    let mut keyring = VaultKeyring::default();
    keyring.insert(first.id, first_key);
    keyring.insert(second.id, second_key);
    let first_cat = vault_category(&pool, &owner, Some(first.id)).await;
    let second_cat = vault_category(&pool, &owner, Some(second.id)).await;
    let plain_cat = vault_category(&pool, &owner, None).await;

    for category_ids in [vec![first_cat, second_cat], vec![first_cat, plain_cat]] {
        let got = db::create_bookmark(
            &pool,
            owner.id,
            CreateBookmark {
                url: "https://mixed.example/".into(),
                category_ids,
                ..Default::default()
            },
            &keyring,
        )
        .await;
        assert!(matches!(got, Err(Error::BadRequest(_))));
    }

    let wrong = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://mixed.example/".into(),
            category_ids: vec![first_cat],
            vault_id: Some(second.id),
            ..Default::default()
        },
        &keyring,
    )
    .await;
    assert!(matches!(wrong, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn a_subcategory_stays_in_its_parents_vault() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, _, _) = db::create_vault(&pool, owner.id, "Nest", "nest-passphrase")
        .await
        .expect("vault");
    let parent = vault_category(&pool, &owner, Some(vault.id)).await;

    let got = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Plain child".into(),
            parent_id: Some(parent),
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await;
    assert!(matches!(got, Err(Error::BadRequest(_))));
}
```

Run: `cargo test -p linkdean-db --test vaults`
Expected: compile errors on `vault_id` in `CreateCategory`.

- [ ] **Step 2: Change the models and repo**

`CreateCategory`: replace `pub is_vault: bool,` with:

```rust
    #[serde(default)]
    pub vault_id: Option<Uuid>,
```

`CreateBookmark`: add:

```rust
    /// For a loose vault bookmark. Categories inside a vault imply their vault.
    #[serde(default)]
    pub vault_id: Option<Uuid>,
```

Update every struct literal:

```bash
python3 - <<'EOF'
import glob, re
for p in glob.glob('crates/**/*.rs', recursive=True):
    s = open(p).read()
    n = re.sub(r'is_vault: (?:true|false),(\s+icon:)', r'vault_id: None,\1', s)
    if n != s:
        open(p, 'w').write(n)
EOF
```

`create_category` in `categories.rs`, replacing the checks down to the insert and removing the temporary lookup from Task 3:

```rust
    if let Some(vault_id) = req.vault_id {
        super::vault::get_user_vault(pool, user_id, vault_id).await?;
    }
    let owner_user_id = if let Some(pid) = req.parent_id {
        let (parent, access) = require_category_write(pool, user_id, pid).await?;
        if parent.vault_id != req.vault_id {
            return Err(Error::BadRequest(
                "a subcategory stays in its parent's vault".into(),
            ));
        }
        if req.vault_id.is_some() && !access.is_owner() {
            return Err(Error::Forbidden);
        }
        // Child under shared folder stays under folder owner's tree.
        parent.owner_user_id
    } else {
        user_id
    };
```

with the insert:

```rust
        INSERT INTO categories (owner_user_id, parent_id, name, is_hidden, is_vault, icon, vault_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
```

binding `req.vault_id.is_some()` for `is_vault` and `req.vault_id` last. Use the exact column list the current query has, adding only `vault_id`.

`create_bookmark` in `bookmarks.rs`, replacing the temporary block from Task 3:

```rust
    let mut vault_id = req.vault_id;
    let mut first = true;
    for cid in &cat_ids {
        let (cat, access) = require_category_write(pool, user_id, *cid).await?;
        if cat.is_vault && !access.is_owner() {
            return Err(Error::Forbidden);
        }
        if first && req.vault_id.is_none() {
            vault_id = cat.vault_id;
        } else if cat.vault_id != vault_id {
            return Err(Error::BadRequest(
                "a bookmark can only sit in categories of its own vault".into(),
            ));
        }
        first = false;
    }
    if let Some(id) = vault_id {
        super::vault::get_user_vault(pool, user_id, id).await?;
    }
```

`set_bookmark_categories`, replacing the `any_vault` / `any_non_vault` block:

```rust
    let (bookmark_vault,): (Option<Uuid>,) =
        sqlx::query_as("SELECT vault_id FROM bookmarks WHERE id = $1")
            .bind(bookmark_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    for id in &unique {
        let (cat, _) = require_category_write(pool, user_id, *id).await?;
        if cat.vault_id != bookmark_vault {
            return Err(Error::BadRequest(
                "a bookmark can only sit in categories of its own vault".into(),
            ));
        }
    }
```

`routes/categories/mod.rs` `create`: delete the `if req.is_vault && ...` block.

- [ ] **Step 3: Add the web data layer**

`apps/web/src/schemas/vault/vault.ts`, added to the existing file:

```ts
export const vaultSchema = z.object({
  id: z.string(),
  name: z.string(),
  unlocked: z.boolean(),
  has_recovery_key: z.boolean(),
  in_global: z.boolean(),
  category_count: z.number().nullish(),
  bookmark_count: z.number().nullish(),
})

export const vaultsEnvelopeSchema = z.object({ vaults: z.array(vaultSchema) })

export const createdVaultSchema = z.object({ vault: vaultSchema, recovery_key: z.string() })

export const recoveryKeySchema = z.object({ recovery_key: z.string() })

export const globalPassphraseSchema = z.object({ enabled: z.boolean() })

export const vaultMoveBlockerSchema = z.object({
  kind: z.enum(['shared', 'public_link', 'other_category', 'dashboard']),
  subject_id: z.string(),
  related_id: z.string().nullish(),
  message: z.string(),
})

export type Vault = z.infer<typeof vaultSchema>
export type VaultMoveBlocker = z.infer<typeof vaultMoveBlockerSchema>
```

`apps/web/src/api/client.ts`, so callers can read a `409` body:

```ts
export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly body: unknown
  ) {
    super(message)
  }
}
```

and in `req`, keep the parsed body and throw it:

```ts
  if (!res.ok) {
    let msg = res.statusText
    let body: unknown
    try {
      body = await res.json()
      if (body && typeof body === 'object' && 'error' in body && typeof (body as { error: unknown }).error === 'string') {
        msg = (body as { error: string }).error
      }
    } catch {
      /* ignore body parse */
    }
    throw new ApiError(msg, res.status, body)
  }
```

`api/api.ts`, in the same object, importing the new schemas and types:

```ts
  vaults: (): Promise<{ vaults: Vault[] }> => req('/vaults', {}, vaultsEnvelopeSchema),

  createVault: (body: { name: string; passphrase: string }): Promise<{ vault: Vault; recovery_key: string }> =>
    req('/vaults', { method: 'POST', body: JSON.stringify(body) }, createdVaultSchema),

  renameVault: (id: string, name: string): Promise<unknown> =>
    req(`/vaults/${id}`, { method: 'PATCH', body: JSON.stringify({ name }) }),

  deleteVault: (id: string, body: { name: string; password?: string }): Promise<unknown> =>
    req(`/vaults/${id}`, { method: 'DELETE', body: JSON.stringify(body) }),

  unlockVault: (id: string, passphrase: string): Promise<unknown> =>
    req(`/vaults/${id}/unlock`, { method: 'POST', body: JSON.stringify({ passphrase }) }),

  lockVault: (id: string): Promise<unknown> => req(`/vaults/${id}/lock`, { method: 'POST' }),

  lockAllVaults: (): Promise<unknown> => req('/vaults/lock-all', { method: 'POST' }),

  recoverVault: (id: string, body: { recovery_key: string; new_passphrase: string }): Promise<unknown> =>
    req(`/vaults/${id}/recover`, { method: 'POST', body: JSON.stringify(body) }),

  newRecoveryKey: (id: string): Promise<{ recovery_key: string }> =>
    req(`/vaults/${id}/recovery-key`, { method: 'POST' }, recoveryKeySchema),

  globalPassphrase: (): Promise<{ enabled: boolean }> => req('/vaults/global', {}, globalPassphraseSchema),

  enableGlobalPassphrase: (passphrase: string): Promise<unknown> =>
    req('/vaults/global', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  changeGlobalPassphrase: (body: { current_passphrase: string; new_passphrase: string }): Promise<unknown> =>
    req('/vaults/global', { method: 'PUT', body: JSON.stringify(body) }),

  disableGlobalPassphrase: (): Promise<unknown> => req('/vaults/global', { method: 'DELETE' }),

  unlockAllVaults: (passphrase: string): Promise<unknown> =>
    req('/vaults/global/unlock', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  joinGlobal: (id: string, passphrase: string): Promise<unknown> =>
    req(`/vaults/${id}/global`, { method: 'POST', body: JSON.stringify({ passphrase }) }),

  leaveGlobal: (id: string): Promise<unknown> => req(`/vaults/${id}/global`, { method: 'DELETE' }),

  moveToVault: (body: {
    category_ids: string[]
    bookmark_ids: string[]
    destination_vault_id: string | null
  }): Promise<unknown> => req('/vaults/move', { method: 'POST', body: JSON.stringify(body) }),
```

`hooks/vault/useVault.ts`, added to the file. `invalidateVaultRelated` also clears `['vaults']`, and each mutation runs it on success:

```ts
export function useVaults(enabled = true): UseQueryResult<{ vaults: Vault[] }> {
  return useQuery({ queryKey: ['vaults'], queryFn: api.vaults, enabled })
}

export function useGlobalPassphrase(): UseQueryResult<{ enabled: boolean }> {
  return useQuery({ queryKey: ['vaults', 'global'], queryFn: api.globalPassphrase })
}

function useVaultAction<T>(action: (input: T) => Promise<unknown>): UseMutationResult<unknown, Error, T> {
  const qc = useQueryClient()
  return useMutation({ mutationFn: action, onSuccess: () => invalidateVaultRelated(qc) })
}

export function useCreateVault(): UseMutationResult<{ vault: Vault; recovery_key: string }, Error, { name: string; passphrase: string }> {
  const qc = useQueryClient()
  return useMutation({ mutationFn: api.createVault, onSuccess: () => invalidateVaultRelated(qc) })
}

export function useNewRecoveryKey(): UseMutationResult<{ recovery_key: string }, Error, string> {
  const qc = useQueryClient()
  return useMutation({ mutationFn: api.newRecoveryKey, onSuccess: () => invalidateVaultRelated(qc) })
}

export const useRenameVault = (): UseMutationResult<unknown, Error, { id: string; name: string }> =>
  useVaultAction(({ id, name }) => api.renameVault(id, name))

export const useDeleteVault = (): UseMutationResult<unknown, Error, { id: string; name: string; password?: string }> =>
  useVaultAction(({ id, ...body }) => api.deleteVault(id, body))

export const useUnlockVault = (): UseMutationResult<unknown, Error, { id: string; passphrase: string }> =>
  useVaultAction(({ id, passphrase }) => api.unlockVault(id, passphrase))

export const useLockVault = (): UseMutationResult<unknown, Error, string> => useVaultAction(api.lockVault)

export const useRecoverVault = (): UseMutationResult<unknown, Error, { id: string; recovery_key: string; new_passphrase: string }> =>
  useVaultAction(({ id, ...body }) => api.recoverVault(id, body))

export const useEnableGlobalPassphrase = (): UseMutationResult<unknown, Error, string> =>
  useVaultAction(api.enableGlobalPassphrase)

export const useChangeGlobalPassphrase = (): UseMutationResult<unknown, Error, { current_passphrase: string; new_passphrase: string }> =>
  useVaultAction(api.changeGlobalPassphrase)

export const useDisableGlobalPassphrase = (): UseMutationResult<unknown, Error, void> =>
  useVaultAction(() => api.disableGlobalPassphrase())

export const useUnlockAllVaults = (): UseMutationResult<unknown, Error, string> => useVaultAction(api.unlockAllVaults)

export const useJoinGlobal = (): UseMutationResult<unknown, Error, { id: string; passphrase: string }> =>
  useVaultAction(({ id, passphrase }) => api.joinGlobal(id, passphrase))

export const useLeaveGlobal = (): UseMutationResult<unknown, Error, string> => useVaultAction(api.leaveGlobal)

export const useMoveToVault = (): UseMutationResult<
  unknown,
  Error,
  { category_ids: string[]; bookmark_ids: string[]; destination_vault_id: string | null }
> => useVaultAction(api.moveToVault)
```

Add `vault_id: z.string().nullish()` to the category and bookmark schemas, beside `is_vault`.

- [ ] **Step 4: Update the drawers**

`CategoryDrawer.tsx`: replace the `vault` boolean state with `vaultId: string | null`, the `Switch` labelled "Vault" with a `Select`:

```tsx
<Select
  label="Vault"
  placeholder="Not in a vault"
  clearable
  disabled={!canCreateVault}
  data={(vaults.data?.vaults ?? []).filter((v) => v.unlocked).map((v) => ({ value: v.id, label: v.name }))}
  value={vaultId}
  onChange={setVaultId}
/>
```

with `const vaults = useVaults()`, and send `vault_id: vaultId` instead of `is_vault: vault`. When a parent is set, send the parent's `vault_id`. Remove `is_vault` from the `api.createCategory` body type.

`BookmarkDrawer.tsx`: when creating, show the same `Select` if `draft.category_ids` is empty, and send `vault_id`. Add `vault_id?: string | null` to the create body type and `emptyDraft`.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p linkdean-db --test vaults`
Expected: 10 passed.

Run the Rust gate and the web gate.

Commit: `Create categories and loose bookmarks in a chosen vault.`

---

### Task 8: Moves and blockers

**Files:** Create `crates/db/src/repo/vault_moves.rs`. Modify `crates/db/src/repo/mod.rs`, `crates/db/src/repo/bookmarks.rs`, `crates/api/src/routes/vault/mod.rs`. Test `crates/db/tests/vault_moves.rs`.

- [ ] **Step 1: Write the failing tests**

`crates/db/tests/vault_moves.rs`:

```rust
//! Moving categories and bookmarks into, out of and between vaults.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, CreateDashboard, Error, MoveToVaultRequest,
    ShareAccess, User, VaultKeyring, VaultMoveBlockerKind, VaultMoveOutcome,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping vault move tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

struct Setup {
    pool: PgPool,
    owner: User,
    vault: Uuid,
    keyring: VaultKeyring,
}

async fn setup() -> Option<Setup> {
    let pool = pool_or_skip().await?;
    let owner = person(&pool, "owner").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Dest", "dest-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    Some(Setup {
        pool,
        owner,
        vault: vault.id,
        keyring,
    })
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@moves.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user")
}

async fn category(s: &Setup, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        &s.pool,
        s.owner.id,
        CreateCategory {
            name: name.into(),
            parent_id,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn bookmark(s: &Setup, url: &str, category_ids: Vec<Uuid>, tags: Vec<String>) -> Uuid {
    db::create_bookmark(
        &s.pool,
        s.owner.id,
        CreateBookmark {
            url: url.into(),
            title: Some("Rust docs".into()),
            notes: Some("private note".into()),
            category_ids,
            tags,
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

fn request(categories: Vec<Uuid>, bookmarks: Vec<Uuid>, to: Option<Uuid>) -> MoveToVaultRequest {
    MoveToVaultRequest {
        category_ids: categories,
        bookmark_ids: bookmarks,
        destination_vault_id: to,
    }
}

async fn blockers(s: &Setup, req: MoveToVaultRequest) -> Vec<VaultMoveBlockerKind> {
    match db::move_to_vault(&s.pool, s.owner.id, req, &s.keyring)
        .await
        .expect("move")
    {
        VaultMoveOutcome::Blocked(found) => found.into_iter().map(|b| b.kind).collect(),
        VaultMoveOutcome::Moved { .. } => panic!("expected blockers"),
    }
}

#[tokio::test]
async fn moving_in_wipes_plaintext_and_moving_out_restores_it() {
    let Some(s) = setup().await else {
        return;
    };
    let id = bookmark(&s, "https://rust.example/", vec![], vec!["lang".into()]).await;
    sqlx::query(
        "UPDATE bookmarks SET favicon_url = '/api/x', preview_image_url = '/api/y', has_preview = TRUE, link_check_status = 'ok' WHERE id = $1",
    )
    .bind(id)
    .execute(&s.pool)
    .await
    .expect("seed");
    sqlx::query(
        "INSERT INTO activity_events (user_id, action, subject_type, subject_id, subject_title, summary) VALUES ($1, 'created', 'bookmark', $2, 'Rust docs', 'Added Rust docs')",
    )
    .bind(s.owner.id)
    .bind(id)
    .execute(&s.pool)
    .await
    .expect("activity");

    let outcome = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move in");
    let VaultMoveOutcome::Moved { left_search, .. } = outcome else {
        panic!("expected a move");
    };
    assert_eq!(left_search, vec![id]);

    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id).await.expect("raw");
    assert!(raw.is_vault);
    assert_eq!(raw.vault_id, Some(s.vault));
    assert_eq!(raw.url, "");
    assert_eq!(raw.title, "");
    assert_eq!(raw.notes, None);
    assert_eq!(raw.favicon_url, None);
    assert_eq!(raw.preview_image_url, None);
    assert!(!raw.has_preview);

    let (tags,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bookmark_tags WHERE bookmark_id = $1")
        .bind(id)
        .fetch_one(&s.pool)
        .await
        .expect("tags");
    assert_eq!(tags, 0);
    let (title, summary): (String, String) = sqlx::query_as(
        "SELECT subject_title, summary FROM activity_events WHERE subject_id = $1",
    )
    .bind(id)
    .fetch_one(&s.pool)
    .await
    .expect("activity");
    assert!(!title.contains("Rust") && !summary.contains("Rust"));

    let view = db::bookmark_to_view(&s.pool, raw, &s.keyring, s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.url, "https://rust.example/");
    assert_eq!(view.notes.as_deref(), Some("private note"));
    assert_eq!(view.tags, vec!["lang".to_string()]);

    let outcome = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], None),
        &s.keyring,
    )
    .await
    .expect("move out");
    let VaultMoveOutcome::Moved { joined_search, .. } = outcome else {
        panic!("expected a move");
    };
    assert_eq!(joined_search, vec![id]);
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id).await.expect("raw");
    assert!(!raw.is_vault);
    assert_eq!(raw.url, "https://rust.example/");
    assert_eq!(raw.vault_ciphertext, None);
    let view = db::bookmark_to_view(&s.pool, raw, &VaultKeyring::default(), s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.tags, vec!["lang".to_string()]);
}

#[tokio::test]
async fn moving_a_category_takes_its_subcategories_and_bookmarks() {
    let Some(s) = setup().await else {
        return;
    };
    let outer = category(&s, "Outer", None).await;
    let parent = category(&s, "Parent", Some(outer)).await;
    let child = category(&s, "Child", Some(parent)).await;
    let id = bookmark(&s, "https://nested.example/", vec![child], vec![]).await;

    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![parent], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move");

    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id).await.expect("raw");
    assert_eq!(raw.vault_id, Some(s.vault));
    let cats = db::list_categories(&s.pool, s.owner.id, false, &s.keyring.vault_ids(), false)
        .await
        .expect("cats");
    let moved = cats.iter().find(|c| c.id == parent).expect("parent");
    assert_eq!(moved.parent_id, None);
    assert!(moved.is_vault);
    assert!(cats.iter().find(|c| c.id == child).expect("child").is_vault);
    assert!(!cats.iter().find(|c| c.id == outer).expect("outer").is_vault);
}

#[tokio::test]
async fn every_blocker_is_listed_and_nothing_moves() {
    let Some(s) = setup().await else {
        return;
    };
    let friend = person(&s.pool, "friend").await;
    let reading = category(&s, "Reading", None).await;
    let work = category(&s, "Work", None).await;
    let id = bookmark(&s, "https://both.example/", vec![reading, work], vec![]).await;

    db::create_share(
        &s.pool,
        s.owner.id,
        reading,
        CreateCategoryShare {
            email: friend.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("invite");
    db::enable_category_public_link(&s.pool, s.owner.id, reading)
        .await
        .expect("public link");
    let board = db::create_dashboard(
        &s.pool,
        s.owner.id,
        CreateDashboard {
            title: Some("Home".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard");
    let (section,): (Uuid,) = sqlx::query_as(
        "INSERT INTO dashboard_sections (dashboard_id, title) VALUES ($1, 'Top') RETURNING id",
    )
    .bind(board.id)
    .fetch_one(&s.pool)
    .await
    .expect("section");
    sqlx::query("INSERT INTO dashboard_items (section_id, bookmark_id) VALUES ($1, $2)")
        .bind(section)
        .bind(id)
        .execute(&s.pool)
        .await
        .expect("pin");

    let mut found = blockers(&s, request(vec![reading], vec![], Some(s.vault))).await;
    found.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(
        found,
        vec![
            VaultMoveBlockerKind::Dashboard,
            VaultMoveBlockerKind::OtherCategory,
            VaultMoveBlockerKind::PublicLink,
            VaultMoveBlockerKind::Shared,
        ]
    );

    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id).await.expect("raw");
    assert!(!raw.is_vault);
    assert_eq!(raw.url, "https://both.example/");
}

#[tokio::test]
async fn moving_both_categories_together_is_not_blocked() {
    let Some(s) = setup().await else {
        return;
    };
    let first = category(&s, "First", None).await;
    let second = category(&s, "Second", None).await;
    let id = bookmark(&s, "https://both.example/", vec![first, second], vec![]).await;

    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![first, second], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move");
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id).await.expect("raw");
    assert_eq!(raw.vault_id, Some(s.vault));
}

#[tokio::test]
async fn moving_between_vaults_reencrypts_and_needs_both_unlocked() {
    let Some(mut s) = setup().await else {
        return;
    };
    let (other, other_key, _) = db::create_vault(&s.pool, s.owner.id, "Other", "other-passphrase")
        .await
        .expect("other");
    let id = bookmark(&s, "https://move.example/", vec![], vec![]).await;
    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move in");

    let locked = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(other.id)),
        &s.keyring,
    )
    .await;
    assert!(matches!(locked, Err(Error::VaultLocked)));

    s.keyring.insert(other.id, other_key);
    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(other.id)),
        &s.keyring,
    )
    .await
    .expect("move across");

    let mut only_other = VaultKeyring::default();
    only_other.insert(other.id, other_key);
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id).await.expect("raw");
    let view = db::bookmark_to_view(&s.pool, raw, &only_other, s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.url, "https://move.example/");
}

#[tokio::test]
async fn another_users_items_cannot_move() {
    let Some(s) = setup().await else {
        return;
    };
    let id = bookmark(&s, "https://mine.example/", vec![], vec![]).await;
    let stranger = person(&s.pool, "stranger").await;
    let (theirs, key, _) = db::create_vault(&s.pool, stranger.id, "Theirs", "their-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(theirs.id, key);

    let got = db::move_to_vault(
        &s.pool,
        stranger.id,
        request(vec![], vec![id], Some(theirs.id)),
        &keyring,
    )
    .await;
    assert!(matches!(got, Err(Error::NotFound)));
}
```

Run: `cargo test -p linkdean-db --test vault_moves`
Expected: compile error, `move_to_vault` not found.

- [ ] **Step 2: Implement**

In `bookmarks.rs`, add a read of plain tags:

```rust
pub(super) async fn bookmark_tag_names(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT t.name FROM tags t
        JOIN bookmark_tags bt ON bt.tag_id = t.id
        WHERE bt.bookmark_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
```

`crates/db/src/repo/vault_moves.rs`:

```rust
use std::collections::HashSet;

use linkdean_core::crypto;
use linkdean_core::{
    Bookmark, Category, Error, MoveToVaultRequest, Result, VaultKeyring, VaultMoveBlocker,
    VaultMoveBlockerKind, VaultMoveOutcome, VaultPayload,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::bookmarks::bookmark_tag_names;
use super::tags::ensure_tags;
use super::vault::get_user_vault;

const SCRUBBED_TITLE: &str = "Vault item";
const SCRUBBED_SUMMARY: &str = "Moved to a vault";

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Categories with all their subcategories, owned by this user.
async fn expand_categories(pool: &PgPool, user_id: Uuid, ids: &[Uuid]) -> Result<Vec<Category>> {
    let rows = sqlx::query_as::<_, Category>(
        r#"
        WITH RECURSIVE tree AS (
            SELECT * FROM categories WHERE id = ANY($1)
            UNION
            SELECT c.* FROM categories c JOIN tree t ON c.parent_id = t.id
        )
        SELECT * FROM tree
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let found: HashSet<Uuid> = rows.iter().map(|c| c.id).collect();
    if ids.iter().any(|id| !found.contains(id)) || rows.iter().any(|c| c.owner_user_id != user_id) {
        return Err(Error::NotFound);
    }
    Ok(rows)
}

/// Requested bookmarks plus everything in the moved categories, trashed included.
async fn collect_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    requested: &[Uuid],
    category_ids: &[Uuid],
) -> Result<Vec<Bookmark>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT DISTINCT b.* FROM bookmarks b
        LEFT JOIN bookmark_categories bc ON bc.bookmark_id = b.id
        WHERE b.id = ANY($1) OR bc.category_id = ANY($2) OR b.category_id = ANY($2)
        "#,
    )
    .bind(requested)
    .bind(category_ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let found: HashSet<Uuid> = rows.iter().map(|b| b.id).collect();
    if requested.iter().any(|id| !found.contains(id))
        || rows.iter().any(|b| b.owner_user_id != user_id)
    {
        return Err(Error::NotFound);
    }
    Ok(rows)
}

/// A bookmark's readable content, whichever side of a vault it is on.
async fn read_payload(
    pool: &PgPool,
    bookmark: &Bookmark,
    keyring: &VaultKeyring,
) -> Result<VaultPayload> {
    let Some(vault_id) = bookmark.vault_id else {
        return Ok(VaultPayload {
            url: bookmark.url.clone(),
            title: bookmark.title.clone(),
            description: bookmark.description.clone(),
            notes: bookmark.notes.clone(),
            tags: bookmark_tag_names(pool, bookmark.id).await?,
        });
    };
    let key = *keyring.get(vault_id).ok_or(Error::VaultLocked)?;
    let ct = bookmark
        .vault_ciphertext
        .clone()
        .ok_or(Error::msg("vault blob missing"))?;
    let nonce = bookmark
        .vault_nonce
        .clone()
        .ok_or(Error::msg("vault nonce missing"))?;
    super::blocking::run_blocking(move || crypto::decrypt_payload(&key, &ct, &nonce)).await
}

async fn find_blockers(
    pool: &PgPool,
    categories: &[Category],
    bookmarks: &[(Bookmark, VaultPayload)],
) -> Result<Vec<VaultMoveBlocker>> {
    let moving: Vec<Uuid> = categories.iter().map(|c| c.id).collect();
    let mut out = Vec::new();

    for category in categories {
        let (people,): (i64,) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM category_shares WHERE category_id = $1)
                + (SELECT COUNT(*) FROM pending_share_invites
                   WHERE subject_type = 'category' AND subject_id = $1
                     AND claimed_at IS NULL AND revoked_at IS NULL)
            "#,
        )
        .bind(category.id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
        if people > 0 {
            let noun = if people == 1 { "person" } else { "people" };
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::Shared,
                subject_id: category.id,
                related_id: None,
                message: format!(
                    "Remove {people} {noun} from sharing on “{}”",
                    category.name
                ),
            });
        }
        if category.public_slug.is_some() {
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::PublicLink,
                subject_id: category.id,
                related_id: None,
                message: format!("Turn off the public link on “{}”", category.name),
            });
        }
    }

    for (bookmark, payload) in bookmarks {
        let others: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT c.id, c.name FROM bookmark_categories bc
            JOIN categories c ON c.id = bc.category_id
            WHERE bc.bookmark_id = $1 AND NOT (c.id = ANY($2))
            ORDER BY c.name
            "#,
        )
        .bind(bookmark.id)
        .bind(&moving)
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
        for (category_id, name) in others {
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::OtherCategory,
                subject_id: bookmark.id,
                related_id: Some(category_id),
                message: format!("Remove “{}” from “{name}”", payload.title),
            });
        }

        let boards: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT DISTINCT d.id, d.title FROM dashboard_items i
            JOIN dashboard_sections s ON s.id = i.section_id
            JOIN dashboards d ON d.id = s.dashboard_id
            WHERE i.bookmark_id = $1 AND d.deleted_at IS NULL
            "#,
        )
        .bind(bookmark.id)
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
        for (dashboard_id, title) in boards {
            out.push(VaultMoveBlocker {
                kind: VaultMoveBlockerKind::Dashboard,
                subject_id: bookmark.id,
                related_id: Some(dashboard_id),
                message: format!(
                    "Remove “{}” from dashboard “{title}”",
                    payload.title
                ),
            });
        }
    }
    Ok(out)
}

async fn write_encrypted(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    vault_id: Uuid,
    ct: Vec<u8>,
    nonce: Vec<u8>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET is_vault = TRUE, vault_id = $1, vault_ciphertext = $2, vault_nonce = $3,
            url = '', title = '', description = '', notes = NULL,
            favicon_url = NULL, preview_image_url = NULL, has_preview = FALSE,
            is_private = FALSE,
            link_check_status = NULL, link_check_status_code = NULL,
            link_check_error = NULL, link_checked_at = NULL, link_check_fail_count = 0,
            updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(vault_id)
    .bind(ct)
    .bind(nonce)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    sqlx::query(
        r#"
        UPDATE activity_events
        SET subject_title = $1, summary = $2, payload = '{}'::jsonb
        WHERE subject_type = 'bookmark' AND subject_id = $3
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(SCRUBBED_SUMMARY)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    sqlx::query(
        r#"
        UPDATE notifications
        SET subject_title = $1, body = $2, payload = '{}'::jsonb
        WHERE subject_type = 'bookmark' AND subject_id = $3
        "#,
    )
    .bind(SCRUBBED_TITLE)
    .bind(SCRUBBED_SUMMARY)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn write_plain(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    payload: &VaultPayload,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET is_vault = FALSE, vault_id = NULL, vault_ciphertext = NULL, vault_nonce = NULL,
            url = $1, title = $2, description = $3, notes = $4, updated_at = now()
        WHERE id = $5
        "#,
    )
    .bind(&payload.url)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.notes)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

pub async fn move_to_vault(
    pool: &PgPool,
    user_id: Uuid,
    req: MoveToVaultRequest,
    keyring: &VaultKeyring,
) -> Result<VaultMoveOutcome> {
    let destination = req.destination_vault_id;
    if let Some(id) = destination {
        get_user_vault(pool, user_id, id).await?;
    }
    let categories = expand_categories(pool, user_id, &req.category_ids).await?;
    let category_ids: Vec<Uuid> = categories.iter().map(|c| c.id).collect();
    let bookmarks = collect_bookmarks(pool, user_id, &req.bookmark_ids, &category_ids).await?;

    if let Some(id) = destination {
        if !keyring.contains(id) {
            return Err(Error::VaultLocked);
        }
    }

    // Items already at the destination stay put.
    let bookmarks: Vec<Bookmark> = bookmarks
        .into_iter()
        .filter(|b| b.vault_id != destination)
        .collect();

    let mut loaded = Vec::with_capacity(bookmarks.len());
    for bookmark in bookmarks {
        let payload = read_payload(pool, &bookmark, keyring).await?;
        loaded.push((bookmark, payload));
    }

    if destination.is_some() {
        let blockers = find_blockers(pool, &categories, &loaded).await?;
        if !blockers.is_empty() {
            return Ok(VaultMoveOutcome::Blocked(blockers));
        }
    }

    // Keep the transaction short: encrypt before it opens.
    let mut encrypted = Vec::new();
    if let Some(id) = destination {
        let key = *keyring.get(id).ok_or(Error::VaultLocked)?;
        for (bookmark, payload) in &loaded {
            let payload = payload.clone();
            let (ct, nonce) =
                super::blocking::run_blocking(move || crypto::encrypt_payload(&key, &payload))
                    .await?;
            encrypted.push((bookmark.id, ct, nonce));
        }
    }

    let mut tag_ids: Vec<(Uuid, Vec<Uuid>)> = Vec::new();
    if destination.is_none() {
        for (bookmark, payload) in &loaded {
            let tags = ensure_tags(pool, user_id, &payload.tags).await?;
            tag_ids.push((bookmark.id, tags.into_iter().map(|t| t.id).collect()));
        }
    }

    let mut tx = pool.begin().await.map_err(db_err)?;

    for category in &categories {
        let keeps_parent = category
            .parent_id
            .map(|parent| category_ids.contains(&parent))
            .unwrap_or(false);
        sqlx::query(
            r#"
            UPDATE categories
            SET is_vault = $1, vault_id = $2,
                parent_id = CASE WHEN $3 THEN parent_id END,
                updated_at = now()
            WHERE id = $4
            "#,
        )
        .bind(destination.is_some())
        .bind(destination)
        .bind(keeps_parent)
        .bind(category.id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }

    for (id, ct, nonce) in encrypted {
        let vault_id = destination.ok_or(Error::msg("move destination missing"))?;
        write_encrypted(&mut tx, id, vault_id, ct, nonce).await?;
    }
    if destination.is_none() {
        for (bookmark, payload) in &loaded {
            write_plain(&mut tx, bookmark.id, payload).await?;
        }
        for (id, tags) in &tag_ids {
            for tag_id in tags {
                sqlx::query(
                    "INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
        }
    }

    tx.commit().await.map_err(db_err)?;

    let was_plain = |b: &Bookmark| !b.is_vault;
    let left_search = if destination.is_some() {
        loaded.iter().filter(|(b, _)| was_plain(b)).map(|(b, _)| b.id).collect()
    } else {
        Vec::new()
    };
    let joined_search = if destination.is_none() {
        loaded.iter().filter(|(b, _)| !was_plain(b)).map(|(b, _)| b.id).collect()
    } else {
        Vec::new()
    };
    Ok(VaultMoveOutcome::Moved {
        left_search,
        joined_search,
    })
}
```

Export the module in `repo/mod.rs` the same way as `vault`.

- [ ] **Step 3: Add the route**

In `crates/api/src/routes/vault/mod.rs`, add `.route("/vaults/move", post(move_items))` above the `/vaults/{id}` routes:

```rust
async fn move_items(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<MoveToVaultRequest>,
) -> ApiResult<Response> {
    let keyring = state.vault_keyring(user.id).await;
    let outcome = db::move_to_vault(&state.pool, user.id, req, &keyring).await?;
    let (left_search, joined_search) = match outcome {
        VaultMoveOutcome::Blocked(blockers) => {
            let body = json!({ "error": "This move is blocked.", "blockers": blockers });
            return Ok((StatusCode::CONFLICT, Json(body)).into_response());
        }
        VaultMoveOutcome::Moved {
            left_search,
            joined_search,
        } => (left_search, joined_search),
    };

    for id in left_search {
        if let Err(e) = state.search.delete_bookmark(id).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili delete failed");
        }
        linkdean_worker::purge_preview_assets(std::path::Path::new(&state.cfg.preview_dir), id).await;
    }
    for id in joined_search {
        let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
        if let Some(view) = db::bookmark_to_view(&state.pool, raw, &keyring, user.id).await? {
            if let Err(e) = state.search.upsert_bookmark(&view).await {
                tracing::warn!(bookmark_id = %id, error = %e, "meili upsert failed");
            }
        }
        let pool = state.pool.clone();
        let search = state.search.clone();
        let opts = state.enrich_opts();
        let uid = user.id;
        tokio::spawn(async move {
            linkdean_worker::enrich_bookmark(&pool, &search, uid, id, &opts).await;
        });
    }
    Ok(Json(json!({ "ok": true })).into_response())
}
```

`state.preview_path` reads the same config field; match its exact name. Import `axum::http::StatusCode` and `axum::response::{IntoResponse, Response}`.

- [ ] **Step 4: Verify and commit**

Run: `cargo test -p linkdean-db --test vault_moves`
Expected: 6 passed.

Run the Rust gate.

Commit: `Move categories and bookmarks between vaults.`

---

### Task 9: Vault settings UI

**Files:** Create `components/vault/VaultList.tsx`, `CreateVaultForm.tsx`, `RecoveryKeyModal.tsx`, `RecoverVaultModal.tsx`, `DeleteVaultModal.tsx`, `GlobalPassphraseCard.tsx`, and tests `RecoveryKeyModal.test.tsx`, `VaultList.test.tsx`. Modify `components/vault/VaultPanel.tsx`, `components/vault/index.ts`, all under `apps/web/src/`.

- [ ] **Step 1: Write the failing tests**

`components/vault/RecoveryKeyModal.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { RecoveryKeyModal } from './RecoveryKeyModal'

describe('RecoveryKeyModal', () => {
  afterEach(cleanup)

  it('needs confirmation before closing', async () => {
    const onDone = vi.fn()
    render(
      <MantineProvider>
        <RecoveryKeyModal recoveryKey="ABCD-EFGH" onDone={onDone} />
      </MantineProvider>
    )
    expect(screen.getByText('ABCD-EFGH')).toBeTruthy()
    const done = screen.getByRole('button', { name: 'Done' })
    expect(done.hasAttribute('disabled') || done.getAttribute('data-disabled') === 'true').toBe(true)
    await userEvent.click(screen.getByLabelText('I saved this recovery key'))
    await userEvent.click(screen.getByRole('button', { name: 'Done' }))
    expect(onDone).toHaveBeenCalled()
  })
})
```

`components/vault/VaultList.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Vault } from '@/schemas'
import { VaultList } from './VaultList'

const unlock = vi.hoisted(() => vi.fn())
const lock = vi.hoisted(() => vi.fn())
const data = vi.hoisted(() => ({ vaults: [] as Vault[] }))
const idle = { mutateAsync: vi.fn(), isPending: false }

vi.mock('@/hooks', () => ({
  useVaults: () => ({ data: { vaults: data.vaults }, isLoading: false, isError: false }),
  useUnlockVault: () => ({ mutateAsync: unlock, isPending: false }),
  useLockVault: () => ({ mutateAsync: lock, isPending: false }),
  useRenameVault: () => idle,
  useNewRecoveryKey: () => idle,
  useJoinGlobal: () => idle,
  useLeaveGlobal: () => idle,
  useGlobalPassphrase: () => ({ data: { enabled: false } }),
}))

vi.mock('./RecoverVaultModal', () => ({ RecoverVaultModal: () => null }))
vi.mock('./DeleteVaultModal', () => ({ DeleteVaultModal: () => null }))

const base: Vault = { id: 'v1', name: 'Work', unlocked: false, has_recovery_key: true, in_global: false }

describe('VaultList', () => {
  afterEach(cleanup)

  it('unlocks a vault with its passphrase', async () => {
    data.vaults = [base]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    await userEvent.type(screen.getByLabelText('Passphrase for Work'), 'work-passphrase')
    await userEvent.click(screen.getByRole('button', { name: 'Unlock' }))
    expect(unlock).toHaveBeenCalledWith({ id: 'v1', passphrase: 'work-passphrase' })
  })

  it('locks an unlocked vault and shows its counts', async () => {
    data.vaults = [{ ...base, unlocked: true, category_count: 2, bookmark_count: 5 }]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    expect(screen.getByText('2 categories, 5 bookmarks')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Lock' }))
    expect(lock).toHaveBeenCalledWith('v1')
  })
})
```

Run: `cd apps/web && pnpm exec vitest run src/components/vault`
Expected: FAIL, missing modules.

- [ ] **Step 2: Implement the recovery key modal**

`components/vault/RecoveryKeyModal.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Alert, Button, Checkbox, Code, CopyButton, Group, Modal, Stack, Text } from '@mantine/core'

type Props = {
  recoveryKey: string
  onDone: () => void
}

/** Shows a recovery key once. The server keeps only a wrapped copy of the vault key. */
export function RecoveryKeyModal({ recoveryKey, onDone }: Props): ReactElement {
  const [saved, setSaved] = useState(false)
  return (
    <Modal
      opened
      onClose={() => {
        if (saved) onDone()
      }}
      title="Save your recovery key"
      withCloseButton={false}
      closeOnClickOutside={false}
      closeOnEscape={false}
    >
      <Stack>
        <Alert color="yellow" variant="light">
          This is shown once. Without this key or your passphrase, nobody can open this vault, including us.
        </Alert>
        <Code block>{recoveryKey}</Code>
        <Group justify="space-between">
          <CopyButton value={recoveryKey}>
            {({ copied, copy }) => (
              <Button variant="default" onClick={copy}>
                {copied ? 'Copied' : 'Copy'}
              </Button>
            )}
          </CopyButton>
          <Text size="xs" c="dimmed">
            Keep it somewhere safe, away from this device.
          </Text>
        </Group>
        <Checkbox
          label="I saved this recovery key"
          checked={saved}
          onChange={(e) => setSaved(e.currentTarget.checked)}
        />
        <Button disabled={!saved} onClick={onDone}>
          Done
        </Button>
      </Stack>
    </Modal>
  )
}
```

- [ ] **Step 3: Implement the list and forms**

`components/vault/VaultList.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Badge, Button, Group, Paper, PasswordInput, Skeleton, Stack, Switch, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLock, IconLockOpen } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  useGlobalPassphrase,
  useJoinGlobal,
  useLeaveGlobal,
  useLockVault,
  useNewRecoveryKey,
  useRenameVault,
  useUnlockVault,
  useVaults,
} from '@/hooks'
import type { Vault } from '@/schemas'
import { DeleteVaultModal } from './DeleteVaultModal'
import { RecoverVaultModal } from './RecoverVaultModal'
import { RecoveryKeyModal } from './RecoveryKeyModal'

const iconSm = { size: 16, stroke: 1.5 } as const

async function report(action: Promise<unknown>, message: string): Promise<void> {
  try {
    await action
    notifications.show({ message })
  } catch (e) {
    notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
  }
}

function countLabel(vault: Vault): string {
  return `${vault.category_count ?? 0} categories, ${vault.bookmark_count ?? 0} bookmarks`
}

function VaultRow({ vault, globalEnabled }: { vault: Vault; globalEnabled: boolean }): ReactElement {
  const [passphrase, setPassphrase] = useState('')
  const [name, setName] = useState(vault.name)
  const [globalPass, setGlobalPass] = useState('')
  const [recovering, setRecovering] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null)
  const unlock = useUnlockVault()
  const lock = useLockVault()
  const rename = useRenameVault()
  const newKey = useNewRecoveryKey()
  const join = useJoinGlobal()
  const leave = useLeaveGlobal()

  async function onUnlock(): Promise<void> {
    await report(unlock.mutateAsync({ id: vault.id, passphrase }), `${vault.name} unlocked`)
    setPassphrase('')
  }

  async function onNewKey(): Promise<void> {
    try {
      const res = await newKey.mutateAsync(vault.id)
      setRecoveryKey(res.recovery_key)
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack gap="sm">
        <Group justify="space-between">
          <Text fw={500}>{vault.name}</Text>
          <Badge color={vault.unlocked ? 'green' : 'gray'} variant="light">
            {vault.unlocked ? 'Unlocked' : 'Locked'}
          </Badge>
        </Group>

        {vault.unlocked ? (
          <>
            <Text size="sm" c="dimmed">
              {countLabel(vault)}
            </Text>
            <Group align="flex-end">
              <TextInput
                label="Name"
                value={name}
                onChange={(e) => setName(e.currentTarget.value)}
                style={{ flex: 1 }}
              />
              <Button
                variant="default"
                disabled={!name.trim() || name === vault.name}
                loading={rename.isPending}
                onClick={() => void report(rename.mutateAsync({ id: vault.id, name }), 'Vault renamed')}
              >
                Rename
              </Button>
            </Group>
            {globalEnabled && !vault.in_global ? (
              <Group align="flex-end">
                <PasswordInput
                  label="Global passphrase"
                  description="Lets the global passphrase open this vault too."
                  value={globalPass}
                  onChange={(e) => setGlobalPass(e.currentTarget.value)}
                  style={{ flex: 1 }}
                />
                <Button
                  variant="default"
                  disabled={globalPass.length < 8}
                  loading={join.isPending}
                  onClick={() => {
                    void report(join.mutateAsync({ id: vault.id, passphrase: globalPass }), 'Added to global unlock')
                    setGlobalPass('')
                  }}
                >
                  Add to global unlock
                </Button>
              </Group>
            ) : null}
            <Group>
              <Button
                variant="light"
                leftSection={<IconLock {...iconSm} />}
                loading={lock.isPending}
                onClick={() => void report(lock.mutateAsync(vault.id), `${vault.name} locked`)}
              >
                Lock
              </Button>
              <Button variant="default" loading={newKey.isPending} onClick={() => void onNewKey()}>
                {vault.has_recovery_key ? 'Replace recovery key' : 'Create recovery key'}
              </Button>
            </Group>
          </>
        ) : (
          <Group align="flex-end">
            <PasswordInput
              label={`Passphrase for ${vault.name}`}
              value={passphrase}
              onChange={(e) => setPassphrase(e.currentTarget.value)}
              style={{ flex: 1 }}
            />
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              disabled={passphrase.length < 8}
              loading={unlock.isPending}
              onClick={() => void onUnlock()}
            >
              Unlock
            </Button>
          </Group>
        )}

        {globalEnabled && vault.in_global ? (
          <Switch
            label="Opens with the global passphrase"
            checked
            onChange={() => void report(leave.mutateAsync(vault.id), 'Removed from global unlock')}
          />
        ) : null}

        <Group justify="space-between">
          {vault.has_recovery_key ? (
            <Button variant="subtle" size="compact-sm" onClick={() => setRecovering(true)}>
              Forgot passphrase
            </Button>
          ) : (
            <span />
          )}
          <Button variant="subtle" color="red" size="compact-sm" onClick={() => setDeleting(true)}>
            Delete vault
          </Button>
        </Group>
      </Stack>

      {recovering ? <RecoverVaultModal vault={vault} onClose={() => setRecovering(false)} /> : null}
      {deleting ? <DeleteVaultModal vault={vault} onClose={() => setDeleting(false)} /> : null}
      {recoveryKey ? <RecoveryKeyModal recoveryKey={recoveryKey} onDone={() => setRecoveryKey(null)} /> : null}
    </Paper>
  )
}

export function VaultList(): ReactElement {
  const list = useVaults()
  const global = useGlobalPassphrase()
  if (list.isError) return <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} />
  if (list.isLoading) return <Skeleton height={120} />
  const vaults = list.data?.vaults ?? []
  if (vaults.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No vaults yet.
      </Text>
    )
  }
  return (
    <Stack>
      {vaults.map((vault) => (
        <VaultRow key={vault.id} vault={vault} globalEnabled={!!global.data?.enabled} />
      ))}
    </Stack>
  )
}
```

`components/vault/CreateVaultForm.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Alert, Button, Paper, PasswordInput, Stack, TextInput, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useCreateVault } from '@/hooks'
import { RecoveryKeyModal } from './RecoveryKeyModal'

export function CreateVaultForm(): ReactElement {
  const [name, setName] = useState('')
  const [passphrase, setPassphrase] = useState('')
  const [confirm, setConfirm] = useState('')
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null)
  const create = useCreateVault()
  const ready = !!name.trim() && passphrase.length >= 8 && passphrase === confirm

  async function onCreate(): Promise<void> {
    try {
      const res = await create.mutateAsync({ name, passphrase })
      setRecoveryKey(res.recovery_key)
      setName('')
      setPassphrase('')
      setConfirm('')
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack>
        <Title order={4}>New vault</Title>
        <TextInput label="Name" value={name} onChange={(e) => setName(e.currentTarget.value)} />
        <PasswordInput label="Passphrase" value={passphrase} onChange={(e) => setPassphrase(e.currentTarget.value)} />
        <PasswordInput label="Confirm passphrase" value={confirm} onChange={(e) => setConfirm(e.currentTarget.value)} />
        <Alert color="yellow" variant="light">
          If you lose both the passphrase and the recovery key, this vault's contents are gone for good.
        </Alert>
        <Button disabled={!ready} loading={create.isPending} onClick={() => void onCreate()}>
          Create vault
        </Button>
      </Stack>
      {recoveryKey ? <RecoveryKeyModal recoveryKey={recoveryKey} onDone={() => setRecoveryKey(null)} /> : null}
    </Paper>
  )
}
```

`components/vault/RecoverVaultModal.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Button, Modal, PasswordInput, Stack, Textarea } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useRecoverVault } from '@/hooks'
import type { Vault } from '@/schemas'

export function RecoverVaultModal({ vault, onClose }: { vault: Vault; onClose: () => void }): ReactElement {
  const [recoveryKey, setRecoveryKey] = useState('')
  const [passphrase, setPassphrase] = useState('')
  const recover = useRecoverVault()

  async function onRecover(): Promise<void> {
    try {
      await recover.mutateAsync({ id: vault.id, recovery_key: recoveryKey, new_passphrase: passphrase })
      notifications.show({ message: `${vault.name} unlocked with a new passphrase` })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={`Recover ${vault.name}`}>
      <Stack>
        <Textarea label="Recovery key" value={recoveryKey} onChange={(e) => setRecoveryKey(e.currentTarget.value)} />
        <PasswordInput label="New passphrase" value={passphrase} onChange={(e) => setPassphrase(e.currentTarget.value)} />
        <Button
          disabled={!recoveryKey.trim() || passphrase.length < 8}
          loading={recover.isPending}
          onClick={() => void onRecover()}
        >
          Set new passphrase
        </Button>
      </Stack>
    </Modal>
  )
}
```

`components/vault/DeleteVaultModal.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Alert, Button, Modal, PasswordInput, Stack, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useDeleteVault } from '@/hooks'
import type { Vault } from '@/schemas'

export function DeleteVaultModal({ vault, onClose }: { vault: Vault; onClose: () => void }): ReactElement {
  const [name, setName] = useState('')
  const [password, setPassword] = useState('')
  const remove = useDeleteVault()

  async function onDelete(): Promise<void> {
    try {
      await remove.mutateAsync({ id: vault.id, name, password: password || undefined })
      notifications.show({ message: 'Vault deleted' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={`Delete ${vault.name}`}>
      <Stack>
        <Alert color="red" variant="light">
          This permanently deletes the vault with all its categories and bookmarks. They cannot be recovered.
        </Alert>
        <TextInput label={`Type ${vault.name} to confirm`} value={name} onChange={(e) => setName(e.currentTarget.value)} />
        <PasswordInput
          label="Account password"
          description="Leave empty if you sign in with single sign-on."
          value={password}
          onChange={(e) => setPassword(e.currentTarget.value)}
        />
        <Button color="red" disabled={name !== vault.name} loading={remove.isPending} onClick={() => void onDelete()}>
          Delete permanently
        </Button>
      </Stack>
    </Modal>
  )
}
```

`components/vault/GlobalPassphraseCard.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Paper, PasswordInput, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  useChangeGlobalPassphrase,
  useDisableGlobalPassphrase,
  useEnableGlobalPassphrase,
  useGlobalPassphrase,
} from '@/hooks'

export function GlobalPassphraseCard(): ReactElement {
  const state = useGlobalPassphrase()
  const enable = useEnableGlobalPassphrase()
  const change = useChangeGlobalPassphrase()
  const disable = useDisableGlobalPassphrase()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const enabled = !!state.data?.enabled

  async function run(action: Promise<unknown>, message: string): Promise<void> {
    try {
      await action
      notifications.show({ message })
      setCurrent('')
      setNext('')
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack>
        <Title order={4}>Global passphrase</Title>
        <Text size="sm" c="dimmed">
          Optional. Opens the vaults you add to it in one step. Each vault keeps its own passphrase.
        </Text>
        <Alert color="yellow" variant="light">
          Anyone who learns this passphrase can open every vault added to it.
        </Alert>
        {enabled ? (
          <PasswordInput label="Current passphrase" value={current} onChange={(e) => setCurrent(e.currentTarget.value)} />
        ) : null}
        <PasswordInput
          label={enabled ? 'New passphrase' : 'Passphrase'}
          value={next}
          onChange={(e) => setNext(e.currentTarget.value)}
        />
        <Group>
          {enabled ? (
            <>
              <Button
                disabled={next.length < 8 || !current}
                loading={change.isPending}
                onClick={() =>
                  void run(
                    change.mutateAsync({ current_passphrase: current, new_passphrase: next }),
                    'Global passphrase changed'
                  )
                }
              >
                Change
              </Button>
              <Button
                variant="default"
                loading={disable.isPending}
                onClick={() => void run(disable.mutateAsync(), 'Global passphrase turned off')}
              >
                Turn off
              </Button>
            </>
          ) : (
            <Button
              disabled={next.length < 8}
              loading={enable.isPending}
              onClick={() => void run(enable.mutateAsync(next), 'Global passphrase turned on')}
            >
              Turn on
            </Button>
          )}
        </Group>
      </Stack>
    </Paper>
  )
}
```

Replace `components/vault/VaultPanel.tsx`:

```tsx
import type { ReactElement } from 'react'
import { Alert, Stack } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { CreateVaultForm } from './CreateVaultForm'
import { GlobalPassphraseCard } from './GlobalPassphraseCard'
import { VaultList } from './VaultList'

export function VaultPanel(): ReactElement {
  return (
    <Stack gap="lg" maw={560}>
      <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
        Each vault has its own passphrase. Bookmark content is encrypted at rest and never reaches search until you unlock the vault.
      </Alert>
      <VaultList />
      <CreateVaultForm />
      <GlobalPassphraseCard />
    </Stack>
  )
}
```

- [ ] **Step 4: Verify and commit**

Run the web gate. Expected: all pass.

Commit: `Replace the vault panel with a vault list.`

---

### Task 10: Move dialog, header and unlock prompts

**Files:** Create `components/vault/MoveToVaultModal.tsx` and `MoveToVaultModal.test.tsx`. Modify `components/bookmark/BookmarkCardMenu.tsx`, `components/bookmark/BookmarkCard.tsx`, `components/bookmark/bookmarkCardTypes.ts`, `components/category/CategoryFolderMenu.tsx`, `pages/library/LibraryBulkBar.tsx`, `pages/library/LibraryPageModals.tsx`, `pages/library/useLibraryPageModel.ts`, `pages/app-shell/AppShellHeader.tsx`, `hooks/library/useLibraryBookmarkEditor.ts`, `hooks/categories/useCategoryFolderActions.tsx`, all under `apps/web/src/`.

- [ ] **Step 1: Write the failing test**

`components/vault/MoveToVaultModal.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api/client'
import { MoveToVaultModal } from './MoveToVaultModal'

const move = vi.hoisted(() => vi.fn())

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, to }: { children: unknown; to: string }) => <a href={to}>{children as string}</a>,
}))

vi.mock('@/hooks', () => ({
  useVaults: () => ({
    data: {
      vaults: [
        { id: 'v1', name: 'Work', unlocked: true, has_recovery_key: true, in_global: false },
        { id: 'v2', name: 'Home', unlocked: false, has_recovery_key: true, in_global: false },
      ],
    },
  }),
  useMoveToVault: () => ({ mutateAsync: move, isPending: false }),
}))

function modal(onClose = vi.fn()) {
  return (
    <MantineProvider>
      <MoveToVaultModal categoryIds={[]} bookmarkIds={['b1']} onClose={onClose} />
    </MantineProvider>
  )
}

describe('MoveToVaultModal', () => {
  beforeEach(() => move.mockReset())
  afterEach(cleanup)

  it('offers only unlocked vaults and moves on confirm', async () => {
    move.mockResolvedValue({ ok: true })
    const onClose = vi.fn()
    render(modal(onClose))
    await userEvent.click(screen.getByLabelText('Destination'))
    expect(screen.queryByRole('option', { name: 'Home' })).toBeNull()
    await userEvent.click(await screen.findByRole('option', { name: 'Work' }))
    await userEvent.click(screen.getByRole('button', { name: 'Move' }))
    expect(move).toHaveBeenCalledWith({ category_ids: [], bookmark_ids: ['b1'], destination_vault_id: 'v1' })
    expect(onClose).toHaveBeenCalled()
  })

  it('lists every blocker and stays open', async () => {
    move.mockRejectedValue(
      new ApiError('This move is blocked.', 409, {
        blockers: [
          { kind: 'shared', subject_id: 'c1', message: 'Remove 2 people from sharing on “Reading”' },
          { kind: 'dashboard', subject_id: 'b1', related_id: 'd1', message: 'Remove “Rust” from dashboard “Home”' },
        ],
      })
    )
    const onClose = vi.fn()
    render(modal(onClose))
    await userEvent.click(screen.getByLabelText('Destination'))
    await userEvent.click(await screen.findByRole('option', { name: 'Work' }))
    await userEvent.click(screen.getByRole('button', { name: 'Move' }))
    expect(await screen.findByText('Remove 2 people from sharing on “Reading”')).toBeTruthy()
    expect(screen.getByText('Remove “Rust” from dashboard “Home”')).toBeTruthy()
    expect(onClose).not.toHaveBeenCalled()
  })
})
```

Run: `cd apps/web && pnpm exec vitest run src/components/vault/MoveToVaultModal.test.tsx`
Expected: FAIL, missing module.

- [ ] **Step 2: Implement the modal**

`components/vault/MoveToVaultModal.tsx`:

```tsx
import { useState, type ReactElement } from 'react'
import { Alert, Anchor, Button, List, Modal, Select, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { Link } from '@tanstack/react-router'
import { z } from 'zod'
import { ApiError } from '@/api/client'
import { useMoveToVault, useVaults } from '@/hooks'
import { vaultMoveBlockerSchema, type VaultMoveBlocker } from '@/schemas'

const OUT_OF_VAULT = 'out'
const blockedBodySchema = z.object({ blockers: z.array(vaultMoveBlockerSchema) })

type Props = {
  categoryIds: string[]
  bookmarkIds: string[]
  onClose: () => void
}

function fixTarget(blocker: VaultMoveBlocker): string {
  return blocker.kind === 'dashboard' ? '/' : '/library'
}

export function MoveToVaultModal({ categoryIds, bookmarkIds, onClose }: Props): ReactElement {
  const vaults = useVaults()
  const move = useMoveToVault()
  const [destination, setDestination] = useState<string | null>(null)
  const [blockers, setBlockers] = useState<VaultMoveBlocker[]>([])

  const options = [
    ...(vaults.data?.vaults ?? []).filter((v) => v.unlocked).map((v) => ({ value: v.id, label: v.name })),
    { value: OUT_OF_VAULT, label: 'Out of vault' },
  ]

  async function onMove(): Promise<void> {
    setBlockers([])
    try {
      await move.mutateAsync({
        category_ids: categoryIds,
        bookmark_ids: bookmarkIds,
        destination_vault_id: destination === OUT_OF_VAULT ? null : destination,
      })
      notifications.show({ message: 'Moved' })
      onClose()
    } catch (e) {
      const parsed = e instanceof ApiError && e.status === 409 ? blockedBodySchema.safeParse(e.body) : null
      if (parsed?.success) {
        setBlockers(parsed.data.blockers)
        return
      }
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title="Move to vault">
      <Stack>
        <Select label="Destination" data={options} value={destination} onChange={setDestination} />
        <Text size="xs" c="dimmed">
          Locked vaults aren't listed. Unlock one first to move items into it.
        </Text>
        {blockers.length > 0 ? (
          <Alert color="yellow" variant="light" title="Fix these first, then move again">
            <List size="sm" spacing="xs">
              {blockers.map((blocker) => (
                <List.Item key={`${blocker.kind}:${blocker.subject_id}:${blocker.related_id ?? ''}`}>
                  <Anchor component={Link} to={fixTarget(blocker)} onClick={onClose}>
                    {blocker.message}
                  </Anchor>
                </List.Item>
              ))}
            </List>
          </Alert>
        ) : null}
        <Button disabled={!destination} loading={move.isPending} onClick={() => void onMove()}>
          Move
        </Button>
      </Stack>
    </Modal>
  )
}
```

Export it from `components/vault/index.ts`.

- [ ] **Step 3: Wire the entry points**

`BookmarkCardMenu.tsx`: add prop `onMoveToVault?: () => void`, and above the `canSaveCopy` block:

```tsx
{canWrite && onMoveToVault ? (
  <Menu.Item leftSection={<IconLock size={14} stroke={1.5} />} onClick={onMoveToVault}>
    Move to vault
  </Menu.Item>
) : null}
```

Pass it through `BookmarkCard.tsx` and `bookmarkCardTypes.ts` the same way as `onDelete`. Only the owner may move, so pass it only when `bookmark.owner_user_id` is the current user.

`useLibraryPageModel.ts`: add state `moveTarget: { categoryIds: string[]; bookmarkIds: string[] } | null` with `setMoveTarget`. `LibraryPageModals.tsx`:

```tsx
{lib.moveTarget ? (
  <MoveToVaultModal
    categoryIds={lib.moveTarget.categoryIds}
    bookmarkIds={lib.moveTarget.bookmarkIds}
    onClose={() => lib.setMoveTarget(null)}
  />
) : null}
```

`LibraryBulkBar.tsx`: add a button beside the Categories button:

```tsx
<Button
  size="xs"
  variant="light"
  disabled={lib.selectedIds.size === 0}
  onClick={() => lib.setMoveTarget({ categoryIds: [], bookmarkIds: [...lib.selectedIds] })}
>
  Move to vault
</Button>
```

`CategoryFolderMenu.tsx`: add prop `onMoveToVault?: () => void`, shown when `isOwner`, using the same `Menu.Item` as above. In `CategoryManager.tsx` `tileMenu`, and `useCategoryFolderActions.tsx`, add `onMoveToVault: () => actions.openMove(cat)`, where `openMove` sets local state rendering `MoveToVaultModal` with `categoryIds={[cat.id]}` and `bookmarkIds={[]}`.

- [ ] **Step 4: Update the header and prompts**

`AppShellHeader.tsx`: below the `Vault unlocked` label, list vaults from a `vaults` prop:

```tsx
{vaults.map((vault) => (
  <Menu.Item
    key={vault.id}
    leftSection={vault.unlocked ? <IconLockOpen {...iconProps} /> : <IconLock {...iconProps} />}
    onClick={() => (vault.unlocked ? onLockVault(vault.id) : onOpenVaultSettings())}
  >
    {vault.name}
  </Menu.Item>
))}
{globalEnabled ? (
  <Menu.Item leftSection={<IconLockOpen {...iconProps} />} onClick={onOpenUnlockAll}>
    Unlock all
  </Menu.Item>
) : null}
```

`AppShell.tsx` provides `vaults` from `useVaults(!!me.data)`, `globalEnabled` from `useGlobalPassphrase`, `onLockVault` from `useLockVault`, `onOpenVaultSettings` navigating to `/settings`, and `onOpenUnlockAll` opening a modal with a `PasswordInput` that calls `useUnlockAllVaults`.

`hooks/library/useLibraryBookmarkEditor.ts`: the prompt lists locked vaults. Replace `useVaultUnlock` with `useUnlockVault` and `useVaults`, add state `unlockVaultId`, and:

```ts
  async function unlockForSearch(): Promise<void> {
    if (!unlockVaultId) return
    try {
      await vaultUnlock.mutateAsync({ id: unlockVaultId, passphrase: vaultPass })
      setVaultPass('')
      setUnlockOpen(false)
      notifications.show({ message: 'Vault unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }
```

`LibraryPageModals.tsx`: add above the passphrase field:

```tsx
<Select
  label="Vault"
  data={lib.lockedVaults.map((v) => ({ value: v.id, label: v.name }))}
  value={lib.unlockVaultId}
  onChange={lib.setUnlockVaultId}
/>
```

and disable Unlock until `lib.unlockVaultId` is set.

- [ ] **Step 5: Verify and commit**

Run the web gate. Expected: all pass.

Commit: `Add moving to a vault from the library.`

---

### Task 11: Remove the old vault API and close out

**Files:** Modify `crates/api/src/routes/vault/mod.rs`, `crates/db/src/repo/vault.rs`, `crates/db/src/repo/users.rs`, `crates/core/src/models/mod.rs`, `crates/core/src/crypto/mod.rs`, `apps/web/src/api/api.ts`, `apps/web/src/hooks/vault/useVault.ts`, `apps/web/src/hooks/prefs/usePrefs.ts`, `apps/web/src/pages/onboarding/OnboardingPage.tsx`, `apps/web/src/schemas/vault/vault.ts`, `docs/ROADMAP.md`, `docs/FEATURES.md`, `docs/ARCHITECTURE.md`.

- [ ] **Step 1: Switch the remaining web callers**

`hooks/prefs/usePrefs.ts`: replace `useVaultLock` with `useLockAllVaults`:

```ts
export function useLockAllVaults(): UseMutationResult<unknown, Error, void> {
  const qc = useQueryClient()
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  return useMutation({
    mutationFn: () => api.lockAllVaults(),
    onSuccess: () => {
      setVaultUnlocked(false)
      invalidateVaultRelated(qc)
    },
  })
}
```

`pages/onboarding/OnboardingPage.tsx`: `setupVault` calls `useCreateVault` with `{ name: 'Vault', passphrase: vaultPass }`, stores `recovery_key`, and renders `RecoveryKeyModal` before moving to the next step:

```tsx
{recoveryKey ? (
  <RecoveryKeyModal
    recoveryKey={recoveryKey}
    onDone={() => {
      setRecoveryKey(null)
      goToNextStep()
    }}
  />
) : null}
```

Use the page's existing step advance for `goToNextStep`.

- [ ] **Step 2: Remove the old code**

Delete `useVaultSetup`, `useVaultUnlock`, `useVaultLock`, `useVaultStatus`, `api.vaultStatus`, `api.vaultSetup`, `api.vaultUnlock`, `api.vaultLock`, `vaultStatusSchema` and `VaultStatus`.

Delete the `/vault/setup`, `/vault/unlock`, `/vault/lock` and `/vault/status` routes and handlers, `VaultSetupRequest`, and `setup_vault`, `get_vault` and `unlock_vault_key` in `repo/vault.rs`. In `crypto/mod.rs`, delete `setup_vault` and `VaultKeys`. `VaultUnlockRequest` stays.

Update `crates/db/tests/authz.rs` to call `db::create_vault(&pool, user_a.id, "Vault", "vault-passphrase")`.

- [ ] **Step 3: Smoke-test the routes**

Use a `smoke` database in the throwaway container with the built API on port 8081:

```bash
export DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:55433/smoke MEILI_URL=http://127.0.0.1:1 MEILI_MASTER_KEY=x APP_URL=http://127.0.0.1:8081 SESSION_SECRET=smoke_session_secret_long_enough BIND=127.0.0.1:8081 AVATAR_DIR=/tmp/smoke-av PREVIEW_DIR=/tmp/smoke-pv BROWSER_WS_URL=http://127.0.0.1:1
```

After registering users A and B, opening registration with `PATCH /admin/platform-settings`, assert:

| Request | Expected |
| --- | --- |
| A: `POST /vaults` twice, different names | 200, each with `recovery_key` |
| A: `POST /vaults` duplicate name | 409 |
| B: `POST /vaults/{A's id}/unlock` | 404 |
| A: `POST /vaults/{id}/lock`, then `POST /bookmarks` with `vault_id` | 423 |
| A: `POST /vaults/{id}/unlock`, wrong passphrase | Existing unlock error |
| A: `POST /vaults/move` for a shared category | 409 with `blockers` |
| A: `POST /vaults/move` for an unshared bookmark | 200 |
| A: `POST /vaults/{id}/recover` with the key | 200 |
| A: `POST /vaults/global`, join one, `lock-all`, `POST /vaults/global/unlock` | `unlocked` lists one id |

- [ ] **Step 4: Run a browser check**

Using Playwright with `qa2@example.com` / `password2` against `http://localhost:5173`:

1. Create two vaults, saving each recovery key.
2. Move a bookmark into one vault.
3. Share a category, move it, and see the blocker checklist.
4. Remove the share and move it.
5. Lock one vault and confirm its items disappear while the other's stay.
6. Recover a passphrase with the recovery key.
7. Turn on the global passphrase, join one vault, lock all, and unlock all.

Remove the test vaults afterwards.

- [ ] **Step 5: Update the docs**

In `docs/ROADMAP.md`, remove the multiple vaults line under Vault & privacy, keeping "Client-held vault key" and "Per-session vault unlock".

In `docs/FEATURES.md`, replace the Vault & privacy list with:

```markdown
- Multiple named vaults, each with its own passphrase and key (Argon2id + AES-GCM)
- Vaults hold categories and loose bookmarks; move items in, out and between vaults, with a checklist when something blocks the move
- Recovery key per vault, shown once; a lost passphrase and recovery key means the contents are gone
- Optional global passphrase that unlocks chosen vaults in one step, alongside each vault's own passphrase
- Vault content never indexed in Meilisearch
- Export never dumps vault bookmarks as plaintext
- Vault unlock in session; optional auto-lock after inactivity (Settings)
```

Update `docs/ARCHITECTURE.md` wherever it says one vault per user.

- [ ] **Step 6: Run everything and commit**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -cE "^crates.*(warning|error)"
cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED"
cd apps/web && pnpm exec tsc -b && pnpm lint && pnpm test && pnpm build
graphify update .
```

Expected: clippy `0`, no failures, build exit 0. Remove the container afterwards.

Commit: `Remove the single vault API and record multiple vaults as shipped.`
