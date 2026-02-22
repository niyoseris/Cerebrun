use sqlx::PgPool;
use uuid::Uuid;
use crate::models::system::*;

pub async fn list_system_models(pool: &PgPool) -> Result<Vec<SystemModel>, sqlx::Error> {
    sqlx::query_as::<_, SystemModel>(
        "SELECT * FROM system_models WHERE is_active = true ORDER BY provider, created_at ASC"
    )
    .fetch_all(pool)
    .await
}

pub async fn add_system_model(
    pool: &PgPool,
    provider: &str,
    model_name: &str,
    display_name: Option<&str>,
) -> Result<SystemModel, sqlx::Error> {
    sqlx::query_as::<_, SystemModel>(
        r#"
        INSERT INTO system_models (provider, model_name, display_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (provider, model_name) DO UPDATE SET is_active = true, display_name = EXCLUDED.display_name
        RETURNING *
        "#
    )
    .bind(provider)
    .bind(model_name)
    .bind(display_name)
    .fetch_one(pool)
    .await
}

pub async fn delete_system_model(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("UPDATE system_models SET is_active = false WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn get_setting(pool: &PgPool, key: &str) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT value FROM system_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn is_auto_embedding_enabled(pool: &PgPool) -> bool {
    get_setting(pool, "auto_embedding")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

pub async fn get_embedding_provider(pool: &PgPool) -> String {
    get_setting(pool, "embedding_provider")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_str().map(|s| s.trim_matches('"').to_string()))
        .unwrap_or_else(|| "openai".to_string())
}

pub async fn set_setting(pool: &PgPool, key: &str, value: serde_json::Value) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO system_settings (key, value, updated_at) VALUES ($1, $2, NOW()) ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()"
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
