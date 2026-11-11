use axum::body::to_bytes;
use axum::response::IntoResponse;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::crypto::vault::{decrypt_vault_data, derive_vault_key, encrypt_vault_data};
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
fn vault_encryption_roundtrips() {
    let key = derive_vault_key("session-secret");
    let plaintext = br#"{"OPENAI_API_KEY":"sk-test","nested":{"token":"abc"}}"#;

    let encrypted = encrypt_vault_data(plaintext, &key).expect("encryption failed");
    assert_ne!(encrypted, plaintext);

    let decrypted = decrypt_vault_data(&encrypted, &key).expect("decryption failed");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn vault_decryption_with_wrong_key_fails() {
    let key = derive_vault_key("session-secret");
    let wrong = derive_vault_key("different-secret");
    let encrypted = encrypt_vault_data(b"secret", &key).unwrap();

    assert!(decrypt_vault_data(&encrypted, &wrong).is_err());
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
