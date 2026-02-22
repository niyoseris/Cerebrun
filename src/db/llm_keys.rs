use sqlx::PgPool;
use uuid::Uuid;

use crate::models::LlmProviderKey;

pub async fn add_provider_key(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    key_name: &str,
    encrypted_key: &[u8],
) -> Result<LlmProviderKey, sqlx::Error> {
    sqlx::query_as::<_, LlmProviderKey>(
        r#"
        INSERT INTO llm_provider_keys (user_id, provider, key_name, encrypted_key, last_validated_at)
        VALUES ($1, $2, $3, $4, NOW())
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(provider)
    .bind(key_name)
    .bind(encrypted_key)
    .fetch_one(pool)
    .await
}

pub async fn list_provider_keys(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<LlmProviderKey>, sqlx::Error> {
    sqlx::query_as::<_, LlmProviderKey>(
        "SELECT * FROM llm_provider_keys WHERE user_id = $1 AND status = 'active' ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_provider_key(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
) -> Result<Option<LlmProviderKey>, sqlx::Error> {
    sqlx::query_as::<_, LlmProviderKey>(
        "SELECT * FROM llm_provider_keys WHERE user_id = $1 AND provider = $2 AND status = 'active' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(pool)
    .await
}

pub async fn delete_provider_key(
    pool: &PgPool,
    key_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM llm_provider_keys WHERE id = $1 AND user_id = $2")
        .bind(key_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_any_embedding_key(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<LlmProviderKey>, sqlx::Error> {
    // Prefer OpenAI for embeddings, then Ollama
    let mut key = get_provider_key(pool, user_id, "openai").await?;
    if key.is_none() {
        key = get_provider_key(pool, user_id, "ollama").await?;
    }
    Ok(key)
}
