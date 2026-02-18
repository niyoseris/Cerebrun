use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ApiKey;

pub async fn create_api_key(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    key_hash: &str,
    permissions: &serde_json::Value,
) -> Result<ApiKey, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        r#"
        INSERT INTO api_keys (user_id, name, key_hash, permissions)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(key_hash)
    .bind(permissions)
    .fetch_one(pool)
    .await
}

pub async fn get_api_key_by_hash(pool: &PgPool, key_hash: &str) -> Result<Option<ApiKey>, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE key_hash = $1 AND status = 'active'",
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await
}

pub async fn list_api_keys(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiKey>, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_api_key(pool: &PgPool, key_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND user_id = $2")
        .bind(key_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn revoke_api_key(pool: &PgPool, key_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE api_keys SET status = 'revoked' WHERE id = $1 AND user_id = $2",
    )
    .bind(key_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_last_used(pool: &PgPool, key_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(key_id)
        .execute(pool)
        .await?;
    Ok(())
}
