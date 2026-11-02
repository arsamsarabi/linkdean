use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use crypto_box::{PublicKey, SecretKey};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::{Error, Result, VaultPayload};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;

/// Shortest new password or vault passphrase. Existing shorter ones still work.
pub const MIN_SECRET_LEN: usize = 12;

/// Argon2id cost for new password hashes and key wraps: 64 MiB, 3 passes, 1 lane.
/// Each hash and wrap records the cost it used, so raising these later keeps old ones working.
const KDF_MEMORY_KIB: u32 = 64 * 1024;
const KDF_PASSES: u32 = 3;
const KDF_LANES: u32 = 1;

/// Key-wrap salts written since KDF costs were recorded: this tag, memory, passes and lanes
/// as big-endian u32s, then the random salt. Older salts are the bare 16 random bytes and
/// used the Argon2 crate defaults (19 MiB, 2 passes, 1 lane).
const KDF_SALT_TAG: &[u8; 4] = b"LDk1";
const KDF_SALT_HEADER: usize = 4 + 12;

fn argon2_with(params: Params) -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn current_params(output_len: Option<usize>) -> Params {
    Params::new(KDF_MEMORY_KIB, KDF_PASSES, KDF_LANES, output_len).expect("valid Argon2 parameters")
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = argon2_with(current_params(None))
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::msg(e.to_string()))?
        .to_string();
    Ok(hash)
}

/// Checks a password against a stored hash, using the cost recorded in that hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| Error::msg(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Whether a stored hash used a weaker cost than new hashes do, so login should replace it.
pub fn password_needs_rehash(hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    let Ok(params) = Params::try_from(&parsed) else {
        return true;
    };
    parsed.algorithm != Algorithm::Argon2id.ident()
        || params.m_cost() < KDF_MEMORY_KIB
        || params.t_cost() < KDF_PASSES
}

/// Spends the same time as checking a real password, for logins with no account to
/// check against, so response time does not reveal which emails exist.
pub fn verify_dummy_password(password: &str) {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let hash = DUMMY
        .get_or_init(|| hash_password("linkdean-dummy-password").expect("hash dummy password"));
    let _ = verify_password(password, hash);
}

pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn unlock_vault(
    passphrase: &str,
    kdf_salt: &[u8],
    wrapped_key: &[u8],
    wrapped_nonce: &[u8],
) -> Result<[u8; KEY_LEN]> {
    let wrapping_key = derive_key(passphrase, kdf_salt)?;
    let plain =
        decrypt(&wrapping_key, wrapped_key, wrapped_nonce).map_err(|_| Error::VaultUnlockFailed)?;
    if plain.len() != KEY_LEN {
        return Err(Error::VaultUnlockFailed);
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&plain);
    Ok(key)
}

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
    let mut kdf_salt = Vec::with_capacity(KDF_SALT_HEADER + SALT_LEN);
    kdf_salt.extend_from_slice(KDF_SALT_TAG);
    for n in [KDF_MEMORY_KIB, KDF_PASSES, KDF_LANES] {
        kdf_salt.extend_from_slice(&n.to_be_bytes());
    }
    kdf_salt.extend_from_slice(&random_bytes(SALT_LEN));
    let wrapping_key = derive_key(secret, &kdf_salt)?;
    let (wrapped_key, wrapped_nonce) = encrypt(&wrapping_key, data_key)?;
    Ok(WrappedKey {
        kdf_salt,
        wrapped_key,
        wrapped_nonce,
    })
}

/// The public half of a master key, so a vault can join without the passphrase.
pub fn master_public_key(master: &[u8; KEY_LEN]) -> Vec<u8> {
    SecretKey::from(*master).public_key().as_bytes().to_vec()
}

/// Seal a vault's data key so that only the master key can open it.
pub fn seal_data_key(public_key: &[u8], data_key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    let public_key: [u8; KEY_LEN] = public_key
        .try_into()
        .map_err(|_| Error::msg("invalid master public key"))?;
    PublicKey::from(public_key)
        .seal(&mut OsRng, data_key)
        .map_err(|e| Error::msg(e.to_string()))
}

pub fn open_sealed_key(master: &[u8; KEY_LEN], sealed: &[u8]) -> Result<[u8; KEY_LEN]> {
    let plain = SecretKey::from(*master)
        .unseal(sealed)
        .map_err(|_| Error::VaultUnlockFailed)?;
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

pub fn encrypt_payload(
    data_key: &[u8; KEY_LEN],
    payload: &VaultPayload,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let bytes = serde_json::to_vec(payload).map_err(|e| Error::msg(e.to_string()))?;
    encrypt(data_key, &bytes)
}

pub fn decrypt_payload(
    data_key: &[u8; KEY_LEN],
    ciphertext: &[u8],
    nonce: &[u8],
) -> Result<VaultPayload> {
    let plain = decrypt(data_key, ciphertext, nonce)?;
    serde_json::from_slice(&plain).map_err(|e| Error::msg(e.to_string()))
}

/// Derives a wrapping key with the cost recorded in `salt` (see `KDF_SALT_TAG`).
fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let (argon, salt) = match salt.strip_prefix(KDF_SALT_TAG.as_slice()) {
        Some(rest) if rest.len() > 12 => {
            let num =
                |i: usize| u32::from_be_bytes([rest[i], rest[i + 1], rest[i + 2], rest[i + 3]]);
            let params = Params::new(num(0), num(4), num(8), Some(KEY_LEN))
                .map_err(|_| Error::VaultUnlockFailed)?;
            (argon2_with(params), &rest[12..])
        }
        _ => (Argon2::default(), salt),
    };
    let mut key = [0u8; KEY_LEN];
    argon
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(key)
}

fn encrypt(key_bytes: &[u8], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| Error::msg(e.to_string()))?;
    let nonce_bytes = random_bytes(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce_bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::msg("invalid nonce length"))?;
    let ciphertext = cipher
        .encrypt(&Nonce::from(nonce), plaintext)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok((ciphertext, nonce_bytes))
}

fn decrypt(key_bytes: &[u8], ciphertext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|_| Error::VaultUnlockFailed)?;
    let nonce: [u8; NONCE_LEN] = nonce_bytes
        .try_into()
        .map_err(|_| Error::VaultUnlockFailed)?;
    cipher
        .decrypt(&Nonce::from(nonce), ciphertext)
        .map_err(|_| Error::VaultUnlockFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultPayload;

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn token_hash_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }

    #[test]
    fn vault_payload_encrypt_decrypt() {
        let data_key = new_data_key();
        let payload = VaultPayload {
            url: "https://example.com".into(),
            title: "Example".into(),
            description: Some("desc".into()),
            notes: None,
            tags: vec!["a".into()],
        };
        let (ct, nonce) = encrypt_payload(&data_key, &payload).unwrap();
        let back = decrypt_payload(&data_key, &ct, &nonce).unwrap();
        assert_eq!(back.url, payload.url);
        assert_eq!(back.title, payload.title);
        assert_eq!(back.tags, payload.tags);
    }

    #[test]
    fn decrypt_rejects_bad_key_or_nonce_length() {
        let bad_nonce = super::decrypt(&[0u8; super::KEY_LEN], b"x", &[0u8; 4]);
        assert!(matches!(bad_nonce, Err(crate::Error::VaultUnlockFailed)));
        let bad_key = super::decrypt(&[0u8; 4], b"x", &[0u8; super::NONCE_LEN]);
        assert!(matches!(bad_key, Err(crate::Error::VaultUnlockFailed)));
    }
}

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
    fn wraps_record_their_cost_and_old_wraps_still_open() {
        let data_key = new_data_key();
        let wrapped = wrap_data_key("passphrase-long", &data_key).unwrap();
        assert!(wrapped.kdf_salt.starts_with(KDF_SALT_TAG));
        assert_eq!(&wrapped.kdf_salt[4..8], &KDF_MEMORY_KIB.to_be_bytes());

        // A wrap from before costs were recorded: bare salt, crate-default cost.
        let salt = random_bytes(SALT_LEN);
        let mut legacy_key = [0u8; KEY_LEN];
        Argon2::default()
            .hash_password_into(b"old-passphrase", &salt, &mut legacy_key)
            .unwrap();
        let (ct, nonce) = encrypt(&legacy_key, &data_key).unwrap();
        assert_eq!(
            unlock_vault("old-passphrase", &salt, &ct, &nonce).unwrap(),
            data_key
        );
    }

    #[test]
    fn old_password_hashes_verify_and_ask_to_be_replaced() {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let old = Argon2::default()
            .hash_password(b"password-long", &salt)
            .unwrap()
            .to_string();
        assert!(verify_password("password-long", &old).unwrap());
        assert!(password_needs_rehash(&old));
        let new = hash_password("password-long").unwrap();
        assert!(verify_password("password-long", &new).unwrap());
        assert!(!password_needs_rehash(&new));
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
    fn a_sealed_key_opens_only_with_its_master_key() {
        let master = [1u8; KEY_LEN];
        let key = [2u8; KEY_LEN];
        let sealed = seal_data_key(&master_public_key(&master), &key).unwrap();
        assert_eq!(open_sealed_key(&master, &sealed).unwrap(), key);
        assert!(matches!(
            open_sealed_key(&[3u8; KEY_LEN], &sealed),
            Err(Error::VaultUnlockFailed)
        ));
    }

    #[test]
    fn a_bad_public_key_is_rejected() {
        assert!(seal_data_key(&[1u8; 8], &[2u8; KEY_LEN]).is_err());
    }

    #[test]
    fn recovery_keys_are_long_and_normalize() {
        let code = new_recovery_key();
        assert_eq!(code.replace('-', "").len(), 52);
        assert_ne!(code, new_recovery_key());
        let messy = format!("  {} ", code.to_ascii_lowercase().replace('-', " "));
        assert_eq!(
            normalize_recovery_key(&messy),
            normalize_recovery_key(&code)
        );
    }
}
