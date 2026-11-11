use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const NONCE_LEN: usize = 12;

/// Marker for the per-user, versioned vault format. Legacy ciphertexts have no
/// prefix (they start directly with the 12-byte nonce).
pub const V2_PREFIX: &[u8; 4] = b"CBR2";

/// Legacy global key: SHA-256 of the session secret. Kept only to decrypt
/// data written before the per-user key migration.
pub fn derive_legacy_key(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Per-user key derived with HKDF-SHA256. `purpose` provides domain separation
/// between the layer-3 vault and third-party LLM provider keys.
pub fn derive_user_key(passphrase: &str, user_id: Uuid, purpose: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(user_id.as_bytes()), passphrase.as_bytes());
    let mut okm = [0u8; 32];
    let info = format!("cerebrun-vault-v2:{}", purpose);
    hk.expand(info.as_bytes(), &mut okm)
        .expect("32 bytes is a valid HKDF output length");
    okm
}

pub fn is_v2(encrypted: &[u8]) -> bool {
    encrypted.starts_with(V2_PREFIX)
}

pub fn encrypt_v2(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let raw = encrypt_raw(data, key)?;
    let mut result = Vec::with_capacity(V2_PREFIX.len() + raw.len());
    result.extend_from_slice(V2_PREFIX);
    result.extend_from_slice(&raw);
    Ok(result)
}

/// Decrypt either the current per-user format or a legacy global-key ciphertext.
pub fn decrypt(
    encrypted: &[u8],
    user_key: &[u8; 32],
    legacy_key: &[u8; 32],
) -> Result<Vec<u8>, String> {
    if is_v2(encrypted) {
        decrypt_raw(&encrypted[V2_PREFIX.len()..], user_key)
    } else {
        decrypt_raw(encrypted, legacy_key)
    }
}

pub fn encrypt_for_user(
    data: &[u8],
    session_secret: &str,
    user_id: Uuid,
    purpose: &str,
) -> Result<Vec<u8>, String> {
    let key = derive_user_key(session_secret, user_id, purpose);
    encrypt_v2(data, &key)
}

pub fn decrypt_for_user(
    encrypted: &[u8],
    session_secret: &str,
    user_id: Uuid,
    purpose: &str,
) -> Result<Vec<u8>, String> {
    let user_key = derive_user_key(session_secret, user_id, purpose);
    let legacy_key = derive_legacy_key(session_secret);
    decrypt(encrypted, &user_key, &legacy_key)
}

/// Raw legacy encryption, used only by the one-time migration to the v2 format.
pub fn encrypt_legacy(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    encrypt_raw(data, key)
}

/// Raw legacy decryption, used only by the one-time migration to the v2 format.
pub fn decrypt_legacy(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    decrypt_raw(encrypted, key)
}

fn encrypt_raw(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(key.into());
    let mut rng = rand::thread_rng();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn decrypt_raw(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted.len() < NONCE_LEN {
        return Err("Invalid encrypted data".to_string());
    }

    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(&encrypted[..NONCE_LEN]);
    let ciphertext = &encrypted[NONCE_LEN..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))
}
