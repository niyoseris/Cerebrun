use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Layer3Vault, VaultConsentRequest};

pub async fn get_vault(pool: &PgPool, user_id: Uuid) -> Result<Option<Layer3Vault>, sqlx::Error> {
    sqlx::query_as::<_, Layer3Vault>("SELECT * FROM layer3_vault WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn upsert_vault(
    pool: &PgPool,
    user_id: Uuid,
    encryption_key_hash: &str,
    encrypted_data: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO layer3_vault (user_id, encryption_key_hash, encrypted_data)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO UPDATE SET
            encryption_key_hash = EXCLUDED.encryption_key_hash,
            encrypted_data = EXCLUDED.encrypted_data,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(encryption_key_hash)
    .bind(encrypted_data)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_consent_request(
    pool: &PgPool,
    user_id: Uuid,
    api_key_id: Uuid,
    reason: &str,
    requested_data: &serde_json::Value,
) -> Result<VaultConsentRequest, sqlx::Error> {
    sqlx::query_as::<_, VaultConsentRequest>(
        r#"
        INSERT INTO vault_consent_requests (user_id, api_key_id, reason, requested_data)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(api_key_id)
    .bind(reason)
    .bind(requested_data)
    .fetch_one(pool)
    .await
}

pub async fn get_pending_consent_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<VaultConsentRequest>, sqlx::Error> {
    sqlx::query_as::<_, VaultConsentRequest>(
        "SELECT * FROM vault_consent_requests WHERE user_id = $1 AND status = 'pending' ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn approve_consent_request(
    pool: &PgPool,
    request_id: Uuid,
    user_id: Uuid,
    vault_token_hash: &str,
    expires_in_hours: i64,
) -> Result<bool, sqlx::Error> {
    let expires_at = Utc::now() + Duration::hours(expires_in_hours);
    let result = sqlx::query(
        r#"
        UPDATE vault_consent_requests SET
            status = 'approved',
            vault_token_hash = $3,
            expires_at = $4,
            resolved_at = NOW()
        WHERE id = $1 AND user_id = $2 AND status = 'pending'
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .bind(vault_token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn deny_consent_request(
    pool: &PgPool,
    request_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE vault_consent_requests SET
            status = 'denied',
            resolved_at = NOW()
        WHERE id = $1 AND user_id = $2 AND status = 'pending'
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn validate_vault_token(
    pool: &PgPool,
    vault_token_hash: &str,
    api_key_id: Uuid,
) -> Result<Option<VaultConsentRequest>, sqlx::Error> {
    sqlx::query_as::<_, VaultConsentRequest>(
        r#"
        SELECT * FROM vault_consent_requests
        WHERE vault_token_hash = $1
          AND api_key_id = $2
          AND status = 'approved'
          AND expires_at > NOW()
        "#,
    )
    .bind(vault_token_hash)
    .bind(api_key_id)
    .fetch_optional(pool)
    .await
}
