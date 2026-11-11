use axum::body::to_bytes;
use axum::response::IntoResponse;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::crypto::vault::{
    decrypt_for_user, derive_legacy_key, derive_user_key, encrypt_for_user, encrypt_legacy, is_v2,
};
use crate::error::AppError;
use crate::models::User;

fn test_user() -> User {
    User {
        id: Uuid::nil(),
        google_id: "google-test".to_string(),
        email: "test@example.com".to_string(),
        display_name: Some("Test".to_string()),
        avatar_url: None,
        is_admin: false,
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn vault_encryption_roundtrips_with_per_user_key() {
    let user = Uuid::new_v4();
    let plaintext = br#"{"OPENAI_API_KEY":"sk-test","nested":{"token":"abc"}}"#;

    let encrypted =
        encrypt_for_user(plaintext, "session-secret", user, "vault").expect("encryption failed");
    assert!(is_v2(&encrypted));
    assert_ne!(encrypted, plaintext);

    let decrypted =
        decrypt_for_user(&encrypted, "session-secret", user, "vault").expect("decryption failed");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn vault_decryption_with_wrong_user_or_purpose_fails() {
    let user = Uuid::new_v4();
    let other = Uuid::new_v4();
    let encrypted = encrypt_for_user(b"secret", "session-secret", user, "vault").unwrap();

    assert!(decrypt_for_user(&encrypted, "session-secret", other, "vault").is_err());
    assert!(decrypt_for_user(&encrypted, "different-secret", user, "vault").is_err());
    assert!(decrypt_for_user(&encrypted, "session-secret", user, "llm").is_err());
}

#[test]
fn legacy_ciphertexts_remain_readable() {
    let user = Uuid::new_v4();
    let legacy_key = derive_legacy_key("session-secret");
    let legacy = encrypt_legacy(b"legacy-secret", &legacy_key).unwrap();
    assert!(!is_v2(&legacy));

    let decrypted = decrypt_for_user(&legacy, "session-secret", user, "vault").unwrap();
    assert_eq!(decrypted, b"legacy-secret");
}

#[test]
fn keys_are_unique_per_user_and_purpose() {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();

    assert_ne!(
        derive_user_key("secret", first, "vault"),
        derive_user_key("secret", second, "vault")
    );
    assert_ne!(
        derive_user_key("secret", first, "vault"),
        derive_user_key("secret", first, "llm")
    );
}

#[test]
fn dashboard_session_has_full_layer_access() {
    let user = test_user();
    let auth = AuthContext {
        user_id: user.id,
        user,
        api_key_id: None,
        permissions: None,
        is_session: true,
    };

    assert!(auth.has_permission("layer1"));
    assert!(auth.has_permission("layer2"));
    assert!(auth.has_permission("vault"));
}

#[test]
fn api_key_layer_permissions_are_checked() {
    let user = test_user();
    let auth = AuthContext {
        user_id: user.id,
        user,
        api_key_id: Some(Uuid::nil()),
        permissions: Some(json!({"layer1": true, "layer2": false})),
        is_session: false,
    };

    assert!(auth.has_permission("layer1"));
    assert!(!auth.has_permission("layer2"));
    assert!(!auth.has_permission("vault"));
}

#[tokio::test]
async fn internal_errors_do_not_leak_details() {
    let response = AppError::Internal("super-secret-db-detail".to_string()).into_response();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8_lossy(&bytes);

    assert!(!body.contains("super-secret-db-detail"));
    assert!(body.contains("Internal server error"));
}
