use crate::crypto::vault::{decrypt_for_user, derive_legacy_key, encrypt_legacy, is_v2};

use super::helpers::*;

#[tokio::test]
async fn legacy_secrets_are_migrated_to_per_user_keys() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "migration@example.com").await;
    let secret = "test-session-secret"; // matches helpers::setup()

    let legacy_key = derive_legacy_key(secret);
    let provider_plaintext = b"sk-legacy-provider-key";
    let vault_plaintext = br#"{"TOKEN":"legacy-vault-value"}"#;
    let provider_cipher = encrypt_legacy(provider_plaintext, &legacy_key).unwrap();
    let vault_cipher = encrypt_legacy(vault_plaintext, &legacy_key).unwrap();

    sqlx::query(
        "INSERT INTO llm_provider_keys (user_id, provider, key_name, encrypted_key)
         VALUES ($1, 'openai', 'legacy', $2)",
    )
    .bind(user_id)
    .bind(&provider_cipher)
    .execute(&env.pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO layer3_vault (user_id, encryption_key_hash, encrypted_data)
         VALUES ($1, 'legacy', $2)",
    )
    .bind(user_id)
    .bind(&vault_cipher)
    .execute(&env.pool)
    .await
    .unwrap();

    let stats = crate::db::secrets::migrate_secrets_to_v2(&env.pool, secret)
        .await
        .unwrap();
    assert_eq!(stats.provider_keys_migrated, 1);
    assert_eq!(stats.vaults_migrated, 1);
    assert_eq!(stats.failed, 0);

    let (provider_row,): (Vec<u8>,) =
        sqlx::query_as("SELECT encrypted_key FROM llm_provider_keys WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&env.pool)
            .await
            .unwrap();
    assert!(is_v2(&provider_row));
    assert_eq!(
        decrypt_for_user(&provider_row, secret, user_id, "llm").unwrap(),
        provider_plaintext.as_slice()
    );

    let (vault_row,): (Vec<u8>,) =
        sqlx::query_as("SELECT encrypted_data FROM layer3_vault WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&env.pool)
            .await
            .unwrap();
    assert!(is_v2(&vault_row));
    assert_eq!(
        decrypt_for_user(&vault_row, secret, user_id, "vault").unwrap(),
        vault_plaintext.as_slice()
    );

    // Running the migration again must be a no-op.
    let second = crate::db::secrets::migrate_secrets_to_v2(&env.pool, secret)
        .await
        .unwrap();
    assert_eq!(second.provider_keys_migrated, 0);
    assert_eq!(second.vaults_migrated, 0);
    assert!(second.skipped >= 2);

    env.cleanup().await;
}
