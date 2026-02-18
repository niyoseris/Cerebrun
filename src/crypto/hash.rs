use rand::Rng;
use sha2::{Digest, Sha256};

pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_random_key(prefix: &str) -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    format!("{}_{}", prefix, hex::encode(bytes))
}

pub fn generate_session_token() -> String {
    generate_random_key("sess")
}

pub fn generate_api_key() -> String {
    generate_random_key("sk_live")
}

pub fn generate_vault_token() -> String {
    generate_random_key("vt")
}
